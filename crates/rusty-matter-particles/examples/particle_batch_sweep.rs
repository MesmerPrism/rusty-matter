//! Emits JSONL timing rows for Matter particle batch execution sweeps.

use std::{env, error::Error, f32::consts::TAU, num::NonZeroUsize, process, time::Instant};

use rusty_matter_mesh::{
    SurfaceDistanceSampler, SurfaceDistanceSamplerConfig, TriangleMeshSurface,
};
use rusty_matter_model::Vec3;
use rusty_matter_particles::{
    ParticleExecutionBackend, ParticleExecutionConfig, ParticleFixedStepConfig, ParticleSet,
    ParticleSimulationDiagnostics, ParticleSimulator, ParticleState, SdfParticleInteractionConfig,
    SdfParticleInteractionMode, SurfaceParticleRuntime, SurfaceParticleRuntimeConfig,
    SurfaceParticleStepDiagnostics,
};

const SCHEMA_ID: &str = "rusty.matter.particles.batch_sweep.v1";
const DEFAULT_FRAMES: usize = 30;
const DEFAULT_WARMUP_FRAMES: usize = 4;
const DEFAULT_GRID_DIVISIONS: usize = 32;

fn main() {
    if let Err(error) = run() {
        eprintln!("particle_batch_sweep failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = SweepOptions::parse(env::args().skip(1))?;
    if options.help {
        print_usage();
        return Ok(());
    }

    for workload in options.workloads() {
        match workload {
            Workload::Surface => {
                for leaf_triangle_count in &options.leaf_triangle_counts {
                    let sampler = benchmark_sampler(options.grid_divisions, *leaf_triangle_count)?;
                    for particle_count in &options.particle_counts {
                        for batch_size in &options.batch_sizes {
                            for backend in backend_cases() {
                                let row = run_surface_case(
                                    &sampler,
                                    *particle_count,
                                    *batch_size,
                                    backend,
                                    options.frames,
                                    options.warmup_frames,
                                )?;
                                println!("{}", row.to_json_line());
                            }
                        }
                    }
                }
            }
            Workload::Neighbor => {
                for particle_count in &options.particle_counts {
                    for batch_size in &options.batch_sizes {
                        for backend in backend_cases() {
                            let row = run_neighbor_case(
                                *particle_count,
                                *batch_size,
                                backend,
                                options.frames,
                                options.warmup_frames,
                            )?;
                            println!("{}", row.to_json_line());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct SweepOptions {
    particle_counts: Vec<usize>,
    batch_sizes: Vec<usize>,
    frames: usize,
    warmup_frames: usize,
    grid_divisions: usize,
    leaf_triangle_counts: Vec<usize>,
    workload: WorkloadSelection,
    help: bool,
}

impl SweepOptions {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            particle_counts: vec![64, 192, 512, 2_048],
            batch_sizes: vec![32, 64, 128, 256, 512],
            frames: DEFAULT_FRAMES,
            warmup_frames: DEFAULT_WARMUP_FRAMES,
            grid_divisions: DEFAULT_GRID_DIVISIONS,
            leaf_triangle_counts: vec![8],
            workload: WorkloadSelection::Surface,
            help: false,
        };

        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => options.help = true,
                "--quick" => {
                    options.particle_counts = vec![64];
                    options.batch_sizes = vec![256];
                    options.frames = 3;
                    options.warmup_frames = 1;
                    options.leaf_triangle_counts = vec![8];
                    options.workload = WorkloadSelection::All;
                }
                "--full" => {
                    if !options.particle_counts.contains(&8_192) {
                        options.particle_counts.push(8_192);
                    }
                }
                "--counts" => {
                    options.particle_counts = parse_usize_list(
                        args.next()
                            .ok_or_else(|| "--counts requires a comma-separated value".to_owned())?
                            .as_str(),
                    )?;
                }
                "--batch-sizes" => {
                    options.batch_sizes = parse_usize_list(
                        args.next()
                            .ok_or_else(|| {
                                "--batch-sizes requires a comma-separated value".to_owned()
                            })?
                            .as_str(),
                    )?;
                    if options.batch_sizes.iter().any(|value| *value == 0) {
                        return Err("--batch-sizes values must be positive".to_owned());
                    }
                }
                "--frames" => {
                    options.frames = parse_positive_usize(
                        args.next()
                            .ok_or_else(|| "--frames requires a value".to_owned())?
                            .as_str(),
                        "--frames",
                    )?;
                }
                "--warmup-frames" => {
                    options.warmup_frames = parse_usize(
                        args.next()
                            .ok_or_else(|| "--warmup-frames requires a value".to_owned())?
                            .as_str(),
                        "--warmup-frames",
                    )?;
                }
                "--grid-divisions" => {
                    options.grid_divisions = parse_positive_usize(
                        args.next()
                            .ok_or_else(|| "--grid-divisions requires a value".to_owned())?
                            .as_str(),
                        "--grid-divisions",
                    )?;
                }
                "--leaf-triangle-counts" => {
                    options.leaf_triangle_counts = parse_usize_list(
                        args.next()
                            .ok_or_else(|| {
                                "--leaf-triangle-counts requires a comma-separated value".to_owned()
                            })?
                            .as_str(),
                    )?;
                    if options.leaf_triangle_counts.iter().any(|value| *value == 0) {
                        return Err("--leaf-triangle-counts values must be positive".to_owned());
                    }
                }
                "--workload" => {
                    options.workload = WorkloadSelection::parse(
                        args.next()
                            .ok_or_else(|| {
                                "--workload requires surface, neighbor, or all".to_owned()
                            })?
                            .as_str(),
                    )?;
                }
                _ => return Err(format!("unknown argument {arg}")),
            }
        }

        if options.particle_counts.is_empty() {
            return Err("--counts must contain at least one value".to_owned());
        }
        if options.batch_sizes.is_empty() {
            return Err("--batch-sizes must contain at least one value".to_owned());
        }
        if options.leaf_triangle_counts.is_empty() {
            return Err("--leaf-triangle-counts must contain at least one value".to_owned());
        }
        Ok(options)
    }

    fn workloads(&self) -> &'static [Workload] {
        self.workload.workloads()
    }
}

#[derive(Clone, Copy, Debug)]
enum WorkloadSelection {
    Surface,
    Neighbor,
    All,
}

impl WorkloadSelection {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "surface" => Ok(Self::Surface),
            "neighbor" => Ok(Self::Neighbor),
            "all" => Ok(Self::All),
            _ => Err(format!("unknown workload {value}")),
        }
    }

    const fn workloads(self) -> &'static [Workload] {
        match self {
            Self::Surface => &[Workload::Surface],
            Self::Neighbor => &[Workload::Neighbor],
            Self::All => &[Workload::Surface, Workload::Neighbor],
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Workload {
    Surface,
    Neighbor,
}

impl Workload {
    const fn marker_value(self) -> &'static str {
        match self {
            Self::Surface => "surface",
            Self::Neighbor => "neighbor",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BackendCase {
    backend: ParticleExecutionBackend,
    max_threads: Option<usize>,
}

fn backend_cases() -> Vec<BackendCase> {
    #[cfg(not(feature = "parallel"))]
    {
        vec![BackendCase {
            backend: ParticleExecutionBackend::Serial,
            max_threads: None,
        }]
    }

    #[cfg(feature = "parallel")]
    {
        let mut cases = vec![BackendCase {
            backend: ParticleExecutionBackend::Serial,
            max_threads: None,
        }];

        for max_threads in [1, 2, 3, 4] {
            cases.push(BackendCase {
                backend: ParticleExecutionBackend::Parallel,
                max_threads: Some(max_threads),
            });
        }

        cases
    }
}

#[derive(Clone, Debug)]
struct SweepRow {
    workload: Workload,
    backend: ParticleExecutionBackend,
    max_threads: Option<usize>,
    batch_size: usize,
    leaf_triangle_count: usize,
    particle_count: usize,
    frames: usize,
    warmup_frames: usize,
    elapsed_micros: u128,
    execution_elapsed_micros: u128,
    chunk_count: usize,
    worker_count: usize,
    closest_samples: usize,
    surface_node_tests: usize,
    surface_leaf_tests: usize,
    surface_triangle_tests: usize,
    neighbor_checks: usize,
    affected_particles: usize,
    rejected_particles: usize,
    clamped_particles: usize,
    max_speed: f32,
}

impl SweepRow {
    fn to_json_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"workload\":\"{}\",\"backend\":\"{}\",\"max_threads\":{},\"batch_size\":{},\"leaf_triangle_count\":{},\"particle_count\":{},\"frames\":{},\"warmup_frames\":{},\"elapsed_micros\":{},\"avg_frame_micros\":{},\"execution_elapsed_micros\":{},\"chunk_count\":{},\"worker_count\":{},\"closest_samples\":{},\"surface_node_tests\":{},\"surface_leaf_tests\":{},\"surface_triangle_tests\":{},\"neighbor_checks\":{},\"affected_particles\":{},\"rejected_particles\":{},\"clamped_particles\":{},\"max_speed\":{:.6}}}",
            SCHEMA_ID,
            self.workload.marker_value(),
            self.backend.marker_value(),
            option_usize_json(self.max_threads),
            self.batch_size,
            self.leaf_triangle_count,
            self.particle_count,
            self.frames,
            self.warmup_frames,
            self.elapsed_micros,
            self.elapsed_micros / self.frames.max(1) as u128,
            self.execution_elapsed_micros,
            self.chunk_count,
            self.worker_count,
            self.closest_samples,
            self.surface_node_tests,
            self.surface_leaf_tests,
            self.surface_triangle_tests,
            self.neighbor_checks,
            self.affected_particles,
            self.rejected_particles,
            self.clamped_particles,
            self.max_speed,
        )
    }
}

fn run_surface_case(
    sampler: &SurfaceDistanceSampler,
    particle_count: usize,
    batch_size: usize,
    backend: BackendCase,
    frames: usize,
    warmup_frames: usize,
) -> Result<SweepRow, Box<dyn Error>> {
    let mut runtime = SurfaceParticleRuntime::new(
        format!("particles.sweep.surface.{particle_count}"),
        SurfaceParticleRuntimeConfig {
            max_substep_seconds: 1.0 / 90.0,
            max_substeps_per_frame: 1,
            execution: execution_config(backend, batch_size),
            ..SurfaceParticleRuntimeConfig::default()
        },
    )?;
    runtime.reset_random_sphere(
        Vec3::new(0.0, 0.0, 0.22),
        particle_count,
        0.7,
        0.01,
        0.5,
        23,
    )?;

    for _ in 0..warmup_frames {
        let _ = runtime.step_against_surface(sampler, 0.5, Vec3::ZERO, 0.7, 1.0 / 90.0);
    }

    let start = Instant::now();
    let mut accumulator = SurfaceAccumulator::default();
    for _ in 0..frames {
        accumulator.merge(runtime.step_against_surface(sampler, 0.5, Vec3::ZERO, 0.7, 1.0 / 90.0));
    }
    let elapsed_micros = start.elapsed().as_micros();

    Ok(SweepRow {
        workload: Workload::Surface,
        backend: backend.backend,
        max_threads: backend.max_threads,
        batch_size,
        leaf_triangle_count: sampler.stats().leaf_triangle_count,
        particle_count,
        frames,
        warmup_frames,
        elapsed_micros,
        execution_elapsed_micros: accumulator.execution_elapsed_micros,
        chunk_count: accumulator.chunk_count,
        worker_count: accumulator.worker_count,
        closest_samples: accumulator.closest_samples,
        surface_node_tests: accumulator.surface_node_tests,
        surface_leaf_tests: accumulator.surface_leaf_tests,
        surface_triangle_tests: accumulator.surface_triangle_tests,
        neighbor_checks: 0,
        affected_particles: accumulator.affected_particles,
        rejected_particles: accumulator.rejected_particles,
        clamped_particles: accumulator.clamped_particles,
        max_speed: accumulator.max_speed,
    })
}

fn run_neighbor_case(
    particle_count: usize,
    batch_size: usize,
    backend: BackendCase,
    frames: usize,
    warmup_frames: usize,
) -> Result<SweepRow, Box<dyn Error>> {
    let mut simulator = ParticleSimulator::new_with_execution(
        neighbor_particle_set(particle_count),
        ParticleFixedStepConfig {
            fixed_step_seconds: 1.0 / 90.0,
            max_steps_per_frame: 1,
            neighbor_radius: 0.085,
            neighbor_repulsion_strength: 0.65,
            ..ParticleFixedStepConfig::default()
        },
        SdfParticleInteractionConfig {
            mode: SdfParticleInteractionMode::Disabled,
            damping: 0.05,
            max_speed: 8.0,
            ..SdfParticleInteractionConfig::default()
        },
        execution_config(backend, batch_size),
    )?;

    for _ in 0..warmup_frames {
        let _ = simulator.step_frame(1.0 / 90.0);
    }

    let start = Instant::now();
    let mut accumulator = NeighborAccumulator::default();
    for _ in 0..frames {
        accumulator.merge(simulator.step_frame(1.0 / 90.0));
    }
    let elapsed_micros = start.elapsed().as_micros();

    Ok(SweepRow {
        workload: Workload::Neighbor,
        backend: backend.backend,
        max_threads: backend.max_threads,
        batch_size,
        leaf_triangle_count: 0,
        particle_count,
        frames,
        warmup_frames,
        elapsed_micros,
        execution_elapsed_micros: accumulator.execution_elapsed_micros,
        chunk_count: accumulator.chunk_count,
        worker_count: accumulator.worker_count,
        closest_samples: 0,
        surface_node_tests: 0,
        surface_leaf_tests: 0,
        surface_triangle_tests: 0,
        neighbor_checks: accumulator.neighbor_checks,
        affected_particles: accumulator.affected_particles,
        rejected_particles: accumulator.rejected_particles,
        clamped_particles: accumulator.clamped_particles,
        max_speed: accumulator.max_speed,
    })
}

#[derive(Clone, Debug, Default)]
struct SurfaceAccumulator {
    execution_elapsed_micros: u128,
    chunk_count: usize,
    worker_count: usize,
    closest_samples: usize,
    surface_node_tests: usize,
    surface_leaf_tests: usize,
    surface_triangle_tests: usize,
    affected_particles: usize,
    rejected_particles: usize,
    clamped_particles: usize,
    max_speed: f32,
}

impl SurfaceAccumulator {
    fn merge(&mut self, diagnostics: SurfaceParticleStepDiagnostics) {
        self.execution_elapsed_micros += diagnostics.execution.elapsed_micros;
        self.chunk_count += diagnostics.execution.chunk_count;
        self.worker_count = self.worker_count.max(diagnostics.execution.worker_count);
        self.closest_samples += diagnostics.closest_samples;
        self.surface_node_tests += diagnostics.surface_node_tests;
        self.surface_leaf_tests += diagnostics.surface_leaf_tests;
        self.surface_triangle_tests += diagnostics.surface_triangle_tests;
        self.affected_particles += diagnostics.affected_particles;
        self.rejected_particles += diagnostics.rejected_particles;
        self.clamped_particles += diagnostics.clamped_particles;
        self.max_speed = self.max_speed.max(diagnostics.max_speed);
    }
}

#[derive(Clone, Debug, Default)]
struct NeighborAccumulator {
    execution_elapsed_micros: u128,
    chunk_count: usize,
    worker_count: usize,
    neighbor_checks: usize,
    affected_particles: usize,
    rejected_particles: usize,
    clamped_particles: usize,
    max_speed: f32,
}

impl NeighborAccumulator {
    fn merge(&mut self, diagnostics: ParticleSimulationDiagnostics) {
        self.execution_elapsed_micros += diagnostics.execution.elapsed_micros;
        self.chunk_count += diagnostics.execution.chunk_count;
        self.worker_count = self.worker_count.max(diagnostics.execution.worker_count);
        self.neighbor_checks += diagnostics.neighbor_checks;
        self.affected_particles += diagnostics.affected_particles;
        self.rejected_particles += diagnostics.rejected_particles;
        self.clamped_particles += diagnostics.clamped_particles;
        self.max_speed = self.max_speed.max(diagnostics.max_speed);
    }
}

fn execution_config(backend: BackendCase, batch_size: usize) -> ParticleExecutionConfig {
    ParticleExecutionConfig {
        backend: backend.backend,
        batch_size: NonZeroUsize::new(batch_size).expect("batch size is validated as non-zero"),
        max_threads: backend.max_threads,
    }
}

fn benchmark_sampler(
    grid_divisions: usize,
    leaf_triangle_count: usize,
) -> Result<SurfaceDistanceSampler, Box<dyn Error>> {
    Ok(
        benchmark_surface(grid_divisions).distance_sampler(SurfaceDistanceSamplerConfig {
            leaf_triangle_count,
            ..SurfaceDistanceSamplerConfig::default()
        })?,
    )
}

fn benchmark_surface(grid_divisions: usize) -> TriangleMeshSurface {
    let row = grid_divisions + 1;
    let mut vertices = Vec::with_capacity(row * row);
    let mut triangles = Vec::with_capacity(grid_divisions * grid_divisions * 2);

    for y in 0..=grid_divisions {
        let v = y as f32 / grid_divisions as f32;
        for x in 0..=grid_divisions {
            let u = x as f32 / grid_divisions as f32;
            let px = u - 0.5;
            let py = v - 0.5;
            let pz = 0.04 * (u * TAU).sin() * (v * TAU).cos();
            vertices.push(Vec3::new(px, py, pz));
        }
    }

    for y in 0..grid_divisions {
        for x in 0..grid_divisions {
            let lower_left = y * row + x;
            let lower_right = lower_left + 1;
            let upper_left = lower_left + row;
            let upper_right = upper_left + 1;
            triangles.push([lower_left as u32, lower_right as u32, upper_left as u32]);
            triangles.push([lower_right as u32, upper_right as u32, upper_left as u32]);
        }
    }

    TriangleMeshSurface::new(
        format!("mesh.particle_batch_sweep.grid_{grid_divisions}"),
        vertices,
        triangles,
    )
}

fn neighbor_particle_set(particle_count: usize) -> ParticleSet {
    let mut particles = ParticleSet::with_capacity("particles.sweep.neighbor", particle_count);
    for index in 0..particle_count {
        let layer = index / 64;
        let lane = index % 64;
        let angle = lane as f32 * 0.618_034 * TAU;
        let radius = 0.18 + (lane % 8) as f32 * 0.008;
        let z = (layer as f32 * 0.017).sin() * 0.12;
        let position = Vec3::new(radius * angle.cos(), radius * angle.sin(), z);
        let mut particle =
            ParticleState::new(format!("particle.neighbor.{index:05}"), position, 0.01);
        particle.velocity = Vec3::new(-position.y, position.x, 0.0) * 0.03;
        particles.push(particle);
    }
    particles
}

fn parse_usize_list(value: &str) -> Result<Vec<usize>, String> {
    value
        .split(',')
        .map(|part| parse_usize(part.trim(), "list value"))
        .collect()
}

fn parse_positive_usize(value: &str, label: &str) -> Result<usize, String> {
    let parsed = parse_usize(value, label)?;
    if parsed == 0 {
        return Err(format!("{label} must be positive"));
    }
    Ok(parsed)
}

fn parse_usize(value: &str, label: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{label} must be an integer: {error}"))
}

fn option_usize_json(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn print_usage() {
    println!(
        "usage: cargo run -p rusty-matter-particles --example particle_batch_sweep -- [--quick] [--full] [--workload surface|neighbor|all] [--counts 64,192,512,2048] [--batch-sizes 32,64,128,256,512] [--leaf-triangle-counts 4,8,16] [--frames N] [--warmup-frames N]"
    );
}
