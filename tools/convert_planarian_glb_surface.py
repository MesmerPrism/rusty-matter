"""Convert a reviewed planarian GLB into a Matter-owned mesh source module.

The generated Rust module stores provenance constants and loads a compact
little-endian Matter surface payload. GLB remains an external/provenance source;
core Matter crates consume only their own mesh contract.
"""

from __future__ import annotations

import argparse
import hashlib
import struct
import sys
from pathlib import Path
from typing import Any

from extract_glb_mesh_surfaces import (
    TRIANGLES_MODE,
    SmokeError,
    bounds_for_positions,
    mesh_surface_index_hash,
    read_glb,
    read_positions,
    read_triangles,
)


EXPECTED_SHA256 = "a170a62ba705a81e73dd7fcfb5808431ff1a0b5c0da6322742c1e2c6ce480dda"
SOURCE_URL = "https://sketchfab.com/3d-models/planaria-8e5a7c4e312e4b08b20676608cb2399f"
LICENSE = "CC-BY-4.0"
ATTRIBUTION = "aphanizomenon, Planaria, Sketchfab"
EVIDENCE_TYPE = "educational_abstraction"
SURFACE_ID = "mesh.planarian_ap.sketchfab_educational_surface"
TARGET_LENGTH = 1.65
Y_OFFSET = -0.09


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate a Matter Rust mesh module from the reviewed Planaria GLB."
    )
    parser.add_argument("--glb", required=True, type=Path, help="Reviewed input .glb")
    parser.add_argument("--output", required=True, type=Path, help="Generated Rust module")
    parser.add_argument(
        "--data-output",
        type=Path,
        help=(
            "Generated little-endian mesh payload. Defaults to "
            "<output-dir>/planarian_mesh_asset/planaria_sketchfab_surface.bin."
        ),
    )
    parser.add_argument("--mesh-index", type=int, default=0, help="GLB mesh index")
    parser.add_argument(
        "--primitive-index",
        type=int,
        default=0,
        help="Triangle primitive index within the mesh",
    )
    parser.add_argument(
        "--allow-sha-mismatch",
        action="store_true",
        help="Generate even if the GLB SHA-256 differs from the reviewed asset.",
    )
    args = parser.parse_args()

    try:
        result = convert(args)
    except SmokeError as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        return 1

    print(
        "[PASS] wrote Planaria surface module "
        f"{result['vertex_count']} vertices, {result['triangle_count']} triangles"
    )
    print(f"[WRITE] {result['output']}")
    print(f"[WRITE] {result['data_output']}")
    return 0


def convert(args: argparse.Namespace) -> dict[str, Any]:
    glb_path = args.glb.resolve()
    if not glb_path.exists():
        raise SmokeError(f"input GLB does not exist: {glb_path}")
    data = glb_path.read_bytes()
    source_sha256 = hashlib.sha256(data).hexdigest()
    if source_sha256 != EXPECTED_SHA256 and not args.allow_sha_mismatch:
        raise SmokeError(
            "input GLB SHA-256 does not match the reviewed Planaria asset: "
            f"{source_sha256}"
        )

    document, bin_chunk = read_glb(data)
    mesh = indexed(document.get("meshes", []), args.mesh_index, "mesh")
    primitive = indexed(mesh.get("primitives", []), args.primitive_index, "primitive")
    if primitive.get("mode", TRIANGLES_MODE) != TRIANGLES_MODE:
        raise SmokeError("selected primitive is not a triangle primitive")
    attributes = primitive.get("attributes", {})
    position_accessor = attributes.get("POSITION")
    if position_accessor is None:
        raise SmokeError("selected primitive is missing POSITION")

    raw_positions = read_positions(document, bin_chunk, position_accessor)
    raw_triangles = read_triangles(document, bin_chunk, primitive, len(raw_positions))
    normalized_positions, transform = normalize_positions(raw_positions)
    normalized_triangles = [tuple(triangle) for triangle in raw_triangles]
    output = args.output.resolve()
    data_output = (
        args.data_output.resolve()
        if args.data_output is not None
        else output.parent / "planarian_mesh_asset" / "planaria_sketchfab_surface.bin"
    )
    write_surface_data(data_output, normalized_positions, normalized_triangles)
    write_rust_module(
        output,
        data_output,
        source_sha256,
        len(data),
        document.get("asset", {}),
        raw_positions,
        normalized_positions,
        normalized_triangles,
        transform,
    )
    return {
        "vertex_count": len(normalized_positions),
        "triangle_count": len(normalized_triangles),
        "output": output,
        "data_output": data_output,
    }


def indexed(items: list[Any], index: int, label: str) -> Any:
    if index < 0 or index >= len(items):
        raise SmokeError(f"{label} index out of range: {index}")
    item = items[index]
    if not isinstance(item, dict):
        raise SmokeError(f"{label} entry is not an object: {index}")
    return item


def normalize_positions(
    raw_positions: list[dict[str, float]]
) -> tuple[list[tuple[float, float, float]], dict[str, Any]]:
    source_bounds_min, source_bounds_max = bounds_for_positions(raw_positions)
    source_spans = [
        source_bounds_max["x"] - source_bounds_min["x"],
        source_bounds_max["y"] - source_bounds_min["y"],
        source_bounds_max["z"] - source_bounds_min["z"],
    ]
    longest_axis = max(range(3), key=lambda index: source_spans[index])
    axis_mapping = "z"

    aligned = []
    for position in raw_positions:
        x = float(position["x"])
        y = float(position["y"])
        z = float(position["z"])
        if longest_axis == 0:
            # Mirrors Three.js root.rotation.y += Math.PI / 2.
            aligned.append((z, y, -x))
            axis_mapping = "x_to_z"
        elif longest_axis == 1:
            # Mirrors Three.js root.rotation.x += Math.PI / 2.
            aligned.append((x, -z, y))
            axis_mapping = "y_to_z"
        else:
            aligned.append((x, y, z))

    aligned_bounds_min, aligned_bounds_max = tuple_bounds(aligned)
    aligned_spans = [
        aligned_bounds_max[0] - aligned_bounds_min[0],
        aligned_bounds_max[1] - aligned_bounds_min[1],
        aligned_bounds_max[2] - aligned_bounds_min[2],
    ]
    longest_span = max(aligned_spans)
    if longest_span <= 0.0:
        raise SmokeError("input GLB has no positive coordinate span")
    scale = TARGET_LENGTH / longest_span
    scaled = [(x * scale, y * scale, z * scale) for x, y, z in aligned]
    scaled_bounds_min, scaled_bounds_max = tuple_bounds(scaled)
    center = tuple(
        (scaled_bounds_min[index] + scaled_bounds_max[index]) * 0.5 for index in range(3)
    )
    normalized = [
        (x - center[0], y - center[1] + Y_OFFSET, z - center[2])
        for x, y, z in scaled
    ]
    normalized_bounds_min, normalized_bounds_max = tuple_bounds(normalized)
    return normalized, {
        "source_bounds_min": dict_to_tuple(source_bounds_min),
        "source_bounds_max": dict_to_tuple(source_bounds_max),
        "axis_mapping": axis_mapping,
        "target_length": TARGET_LENGTH,
        "scale": scale,
        "center_after_scale": center,
        "y_offset": Y_OFFSET,
        "normalized_bounds_min": normalized_bounds_min,
        "normalized_bounds_max": normalized_bounds_max,
    }


def tuple_bounds(
    positions: list[tuple[float, float, float]]
) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    if not positions:
        raise SmokeError("position buffer is empty")
    min_position = list(positions[0])
    max_position = list(positions[0])
    for position in positions[1:]:
        for axis in range(3):
            min_position[axis] = min(min_position[axis], position[axis])
            max_position[axis] = max(max_position[axis], position[axis])
    return tuple(min_position), tuple(max_position)


def dict_to_tuple(position: dict[str, float]) -> tuple[float, float, float]:
    return (position["x"], position["y"], position["z"])


def write_surface_data(
    output: Path,
    positions: list[tuple[float, float, float]],
    triangles: list[tuple[int, int, int]],
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as file:
        for position in positions:
            file.write(struct.pack("<fff", *position))
        for triangle in triangles:
            file.write(struct.pack("<III", *triangle))


def write_rust_module(
    output: Path,
    data_output: Path,
    source_sha256: str,
    source_size_bytes: int,
    asset: dict[str, Any],
    raw_positions: list[dict[str, float]],
    positions: list[tuple[float, float, float]],
    triangles: list[tuple[int, int, int]],
    transform: dict[str, Any],
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    generator = str(Path(__file__).name)
    try:
        data_include_path = data_output.resolve().relative_to(output.parent.resolve()).as_posix()
    except ValueError as exc:
        raise SmokeError(
            "generated Planaria mesh data must live under the Rust module directory "
            f"for public-safe include_bytes!: {data_output}"
        ) from exc
    lines: list[str] = [
        "// @generated by tools/convert_planarian_glb_surface.py; do not edit by hand.",
        "use rusty_matter_mesh::TriangleMeshSurface;",
        "use rusty_matter_model::Vec3;",
        "",
        f'pub(crate) const PLANARIA_SKETCHFAB_SURFACE_ID: &str = "{SURFACE_ID}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_SOURCE_URL: &str = "{SOURCE_URL}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_LICENSE: &str = "{LICENSE}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_ATTRIBUTION: &str = "{ATTRIBUTION}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_EVIDENCE_TYPE: &str = "{EVIDENCE_TYPE}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_SOURCE_SHA256: &str = "{source_sha256}";',
        f"pub(crate) const PLANARIA_SKETCHFAB_SOURCE_SIZE_BYTES: usize = {source_size_bytes};",
        f"pub(crate) const PLANARIA_SKETCHFAB_VERTEX_COUNT: usize = {len(positions)};",
        f"pub(crate) const PLANARIA_SKETCHFAB_TRIANGLE_COUNT: usize = {len(triangles)};",
        f"pub(crate) const PLANARIA_SKETCHFAB_TOPOLOGY_INDEX_HASH: u64 = {mesh_surface_index_hash([list(triangle) for triangle in triangles])};",
        f'pub(crate) const PLANARIA_SKETCHFAB_GENERATOR: &str = "{escape_string(generator)}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_GLTF_GENERATOR: &str = "{escape_string(str(asset.get("generator", "")))}";',
        f'pub(crate) const PLANARIA_SKETCHFAB_AXIS_MAPPING: &str = "{transform["axis_mapping"]}";',
        f"pub(crate) const PLANARIA_SKETCHFAB_TARGET_LENGTH: f32 = {format_f32(transform['target_length'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_SCALE: f32 = {format_f32(transform['scale'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_CENTER_AFTER_SCALE: [f32; 3] = {format_triplet(transform['center_after_scale'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_Y_OFFSET: f32 = {format_f32(transform['y_offset'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_SOURCE_BOUNDS_MIN: [f32; 3] = {format_triplet(transform['source_bounds_min'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_SOURCE_BOUNDS_MAX: [f32; 3] = {format_triplet(transform['source_bounds_max'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_BOUNDS_MIN: [f32; 3] = {format_triplet(transform['normalized_bounds_min'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_BOUNDS_MAX: [f32; 3] = {format_triplet(transform['normalized_bounds_max'])};",
        f"pub(crate) const PLANARIA_SKETCHFAB_SOURCE_VERTEX_COUNT: usize = {len(raw_positions)};",
        "",
        "const PLANARIA_SKETCHFAB_SURFACE_BYTES: &[u8] =",
        f'    include_bytes!("{escape_string(data_include_path)}");',
        "const PLANARIA_SKETCHFAB_POSITION_BYTES: usize = PLANARIA_SKETCHFAB_VERTEX_COUNT * 3 * 4;",
        "const PLANARIA_SKETCHFAB_TRIANGLE_BYTES: usize = PLANARIA_SKETCHFAB_TRIANGLE_COUNT * 3 * 4;",
        "const PLANARIA_SKETCHFAB_SURFACE_BYTES_LEN: usize =",
        "    PLANARIA_SKETCHFAB_POSITION_BYTES + PLANARIA_SKETCHFAB_TRIANGLE_BYTES;",
        "",
        "pub(crate) fn sketchfab_planaria_surface() -> TriangleMeshSurface {",
        "    assert_eq!(",
        "        PLANARIA_SKETCHFAB_SURFACE_BYTES.len(),",
        "        PLANARIA_SKETCHFAB_SURFACE_BYTES_LEN,",
        '        "generated Planaria surface binary length must match declared counts",',
        "    );",
        "",
        "    let positions = PLANARIA_SKETCHFAB_SURFACE_BYTES[..PLANARIA_SKETCHFAB_POSITION_BYTES]",
        "        .chunks_exact(12)",
        "        .map(|chunk| Vec3::new(read_f32(chunk, 0), read_f32(chunk, 4), read_f32(chunk, 8)))",
        "        .collect();",
        "    let triangles = PLANARIA_SKETCHFAB_SURFACE_BYTES[PLANARIA_SKETCHFAB_POSITION_BYTES..]",
        "        .chunks_exact(12)",
        "        .map(|chunk| [read_u32(chunk, 0), read_u32(chunk, 4), read_u32(chunk, 8)])",
        "        .collect();",
        "",
        "    TriangleMeshSurface::new(PLANARIA_SKETCHFAB_SURFACE_ID, positions, triangles)",
        "}",
        "",
        "fn read_f32(chunk: &[u8], offset: usize) -> f32 {",
        "    let mut bytes = [0_u8; 4];",
        "    bytes.copy_from_slice(&chunk[offset..offset + 4]);",
        "    f32::from_le_bytes(bytes)",
        "}",
        "",
        "fn read_u32(chunk: &[u8], offset: usize) -> u32 {",
        "    let mut bytes = [0_u8; 4];",
        "    bytes.copy_from_slice(&chunk[offset..offset + 4]);",
        "    u32::from_le_bytes(bytes)",
        "}",
    ]
    output.write_text("\n".join(lines), encoding="utf-8", newline="\n")


def format_triplet(values: tuple[float, float, float]) -> str:
    return f"[{format_f32(values[0])}, {format_f32(values[1])}, {format_f32(values[2])}]"


def format_f32(value: float) -> str:
    text = f"{value:.9g}"
    if "e" not in text and "." not in text:
        text += ".0"
    return text


def escape_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


if __name__ == "__main__":
    sys.exit(main())
