"""Extract animated glTF skinned mesh frames as Matter mesh surface positions.

This tool is intentionally kept outside Matter core crates. It evaluates a GLB
recording into generic triangle-surface frames so browser and renderer smoke
tests can recompute SDF/collision from the current mesh pose instead of reading
precomputed field or collider frames.
"""

from __future__ import annotations

import argparse
import bisect
import hashlib
import math
import sys
from pathlib import Path
from typing import Any

from extract_glb_mesh_surfaces import (
    SUMMARY_SCHEMA_ID,
    TRIANGLE_MESH_SURFACE_SCHEMA_ID,
    SmokeError,
    accessor_at,
    bounds_for_positions,
    mesh_surface_index_hash,
    read_accessor,
    read_glb,
    read_triangles,
    safe_token,
    write_json,
)


SEQUENCE_SCHEMA_ID = "rusty.matter.tools.glb_mesh_surface_sequence.v1"


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Extract one animated GLB skinned mesh into a Matter surface "
            "sequence with deformed positions only."
        )
    )
    parser.add_argument("--glb", required=True, type=Path, help="Input .glb file")
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Output surface sequence JSON path",
    )
    parser.add_argument("--mesh-index", type=int, default=0, help="Mesh index to sample")
    parser.add_argument(
        "--primitive-index",
        type=int,
        default=0,
        help="Triangle primitive index within the mesh",
    )
    parser.add_argument(
        "--animation-index",
        type=int,
        default=0,
        help="Animation index to sample",
    )
    parser.add_argument(
        "--frame-count",
        type=int,
        default=120,
        help="Number of sampled animation frames",
    )
    parser.add_argument(
        "--surface-id-prefix",
        default=None,
        help="Optional surface id prefix for generated frames",
    )
    args = parser.parse_args()

    try:
        result = extract_sequence(
            args.glb,
            args.output,
            args.mesh_index,
            args.primitive_index,
            args.animation_index,
            args.frame_count,
            args.surface_id_prefix,
        )
    except SmokeError as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        return 1

    print(
        "[PASS] extracted animated surface sequence "
        f"{result['frame_count']} frames, "
        f"{result['vertex_count']} vertices, "
        f"{result['triangle_count']} triangles"
    )
    print(f"[WRITE] {result['output']}")
    return 0


def extract_sequence(
    glb_path: Path,
    output: Path,
    mesh_index: int,
    primitive_index: int,
    animation_index: int,
    frame_count: int,
    surface_id_prefix: str | None,
) -> dict[str, Any]:
    if frame_count <= 1:
        raise SmokeError("--frame-count must be greater than one")
    glb_path = glb_path.resolve()
    if not glb_path.exists():
        raise SmokeError(f"input GLB does not exist: {glb_path}")

    data = glb_path.read_bytes()
    document, bin_chunk = read_glb(data)
    mesh = indexed(document.get("meshes", []), mesh_index, "mesh")
    primitive = indexed(mesh.get("primitives", []), primitive_index, "primitive")
    attributes = primitive.get("attributes", {})
    for attribute in ("POSITION", "JOINTS_0", "WEIGHTS_0"):
        if attribute not in attributes:
            raise SmokeError(f"primitive is missing required {attribute} attribute")

    mesh_name = mesh.get("name") or f"mesh_{mesh_index:02d}"
    mesh_token = safe_token(mesh_name)
    positions = [tuple(map(float, value)) for value in read_accessor(document, bin_chunk, attributes["POSITION"])]
    joints = read_accessor(document, bin_chunk, attributes["JOINTS_0"])
    weights = read_accessor(document, bin_chunk, attributes["WEIGHTS_0"])
    if len(positions) != len(joints) or len(positions) != len(weights):
        raise SmokeError("POSITION, JOINTS_0, and WEIGHTS_0 accessors must have equal counts")
    triangles = read_triangles(document, bin_chunk, primitive, len(positions))

    node_index, node = find_skinned_mesh_node(document, mesh_index)
    skin_index = node.get("skin")
    if skin_index is None:
        raise SmokeError(f"mesh node {node_index} does not reference a skin")
    skin = indexed(document.get("skins", []), skin_index, "skin")
    joint_nodes = skin.get("joints", [])
    if not joint_nodes:
        raise SmokeError(f"skin {skin_index} has no joints")
    inverse_bind_matrices = read_inverse_bind_matrices(document, bin_chunk, skin, len(joint_nodes))

    animation = indexed(document.get("animations", []), animation_index, "animation")
    animation_tracks = read_animation_tracks(document, bin_chunk, animation)
    duration_seconds = animation_duration(animation_tracks)
    if duration_seconds <= 0.0:
        raise SmokeError("animation has no positive duration")

    parents = node_parents(document)
    base_locals = [node_local_matrix(source_node) for source_node in document.get("nodes", [])]
    prefix = surface_id_prefix or f"mesh.glb_surface_sequence.{mesh_index:02d}.{primitive_index:02d}.{mesh_token}"
    topology_index_hash = mesh_surface_index_hash(triangles)

    frames: list[dict[str, Any]] = []
    sequence_bounds_min: dict[str, float] | None = None
    sequence_bounds_max: dict[str, float] | None = None
    for frame_index in range(frame_count):
        # Do not sample the exact end time; the browser loops frame N-1 -> 0.
        time_seconds = duration_seconds * frame_index / frame_count
        local_matrices = animated_local_matrices(
            document,
            base_locals,
            animation_tracks,
            time_seconds,
        )
        world_matrices = world_matrices_for_nodes(local_matrices, parents)
        mesh_inverse = invert_affine(world_matrices[node_index])
        skin_matrices = [
            mat4_mul(mat4_mul(mesh_inverse, world_matrices[joint_node]), inverse_bind)
            for joint_node, inverse_bind in zip(joint_nodes, inverse_bind_matrices)
        ]
        frame_positions = skin_positions(positions, joints, weights, skin_matrices)
        bounds_min, bounds_max = bounds_for_positions(frame_positions)
        sequence_bounds_min, sequence_bounds_max = merge_bounds(
            sequence_bounds_min,
            sequence_bounds_max,
            bounds_min,
            bounds_max,
        )
        frames.append(
            {
                "frame_index": frame_index,
                "time_seconds": round_float(time_seconds),
                "surface_id": f"{prefix}.frame.{frame_index:04}",
                "positions": frame_positions,
                "bounds_min": bounds_min,
                "bounds_max": bounds_max,
            }
        )

    payload = {
        "schema_id": SEQUENCE_SCHEMA_ID,
        "sequence_id": prefix,
        "source_file_name": glb_path.name,
        "source_size_bytes": len(data),
        "source_sha256": hashlib.sha256(data).hexdigest(),
        "source_summary_schema_id": SUMMARY_SCHEMA_ID,
        "surface_schema_id": TRIANGLE_MESH_SURFACE_SCHEMA_ID,
        "mesh_index": mesh_index,
        "mesh_name": mesh_name,
        "primitive_index": primitive_index,
        "node_index": node_index,
        "node_name": node.get("name") or f"node_{node_index:02d}",
        "skin_index": skin_index,
        "skin_name": skin.get("name") or f"skin_{skin_index:02d}",
        "animation_index": animation_index,
        "animation_name": animation.get("name") or f"animation_{animation_index:02d}",
        "duration_seconds": round_float(duration_seconds),
        "frame_count": frame_count,
        "frame_rate_hz": round_float(frame_count / duration_seconds),
        "vertex_count": len(positions),
        "triangle_count": len(triangles),
        "topology_index_hash": topology_index_hash,
        "bounds_min": sequence_bounds_min,
        "bounds_max": sequence_bounds_max,
        "triangles": triangles,
        "frames": frames,
    }
    validate_sequence(payload)
    write_json(output, payload)
    return {
        "frame_count": frame_count,
        "vertex_count": len(positions),
        "triangle_count": len(triangles),
        "output": output,
    }


def indexed(items: list[Any], index: int, label: str) -> Any:
    if index < 0 or index >= len(items):
        raise SmokeError(f"{label} index out of range: {index}")
    return items[index]


def find_skinned_mesh_node(document: dict[str, Any], mesh_index: int) -> tuple[int, dict[str, Any]]:
    for node_index, node in enumerate(document.get("nodes", [])):
        if node.get("mesh") == mesh_index and "skin" in node:
            return node_index, node
    raise SmokeError(f"no skinned node references mesh {mesh_index}")


def read_inverse_bind_matrices(
    document: dict[str, Any],
    bin_chunk: bytes,
    skin: dict[str, Any],
    joint_count: int,
) -> list[list[float]]:
    accessor_index = skin.get("inverseBindMatrices")
    if accessor_index is None:
        return [identity_mat4() for _ in range(joint_count)]
    accessor = accessor_at(document, accessor_index)
    if accessor.get("componentType") != 5126 or accessor.get("type") != "MAT4":
        raise SmokeError("inverseBindMatrices accessor must be componentType 5126 MAT4")
    values = read_accessor(document, bin_chunk, accessor_index)
    if len(values) != joint_count:
        raise SmokeError("inverseBindMatrices count must match skin joint count")
    return [gltf_mat4_to_row_major(value) for value in values]


def read_animation_tracks(
    document: dict[str, Any],
    bin_chunk: bytes,
    animation: dict[str, Any],
) -> dict[int, dict[str, dict[str, Any]]]:
    tracks: dict[int, dict[str, dict[str, Any]]] = {}
    samplers = animation.get("samplers", [])
    for channel in animation.get("channels", []):
        sampler = indexed(samplers, channel.get("sampler", -1), "animation sampler")
        target = channel.get("target", {})
        node_index = target.get("node")
        path = target.get("path")
        if node_index is None or path not in {"translation", "rotation", "scale"}:
            continue
        input_accessor = sampler.get("input")
        output_accessor = sampler.get("output")
        if input_accessor is None or output_accessor is None:
            raise SmokeError("animation sampler is missing input or output accessor")
        times = [float(value[0]) for value in read_accessor(document, bin_chunk, input_accessor)]
        values = [tuple(map(float, value)) for value in read_accessor(document, bin_chunk, output_accessor)]
        if len(times) != len(values) or not times:
            raise SmokeError("animation sampler input/output counts must match and be non-empty")
        tracks.setdefault(int(node_index), {})[str(path)] = {
            "times": times,
            "values": values,
            "interpolation": sampler.get("interpolation", "LINEAR"),
        }
    return tracks


def animation_duration(tracks: dict[int, dict[str, dict[str, Any]]]) -> float:
    duration = 0.0
    for node_tracks in tracks.values():
        for track in node_tracks.values():
            duration = max(duration, max(track["times"]))
    return duration


def node_parents(document: dict[str, Any]) -> list[int | None]:
    nodes = document.get("nodes", [])
    parents: list[int | None] = [None for _ in nodes]
    for node_index, node in enumerate(nodes):
        for child_index in node.get("children", []):
            if child_index < 0 or child_index >= len(nodes):
                raise SmokeError(f"child node index out of range: {child_index}")
            parents[child_index] = node_index
    return parents


def animated_local_matrices(
    document: dict[str, Any],
    base_locals: list[list[float]],
    tracks: dict[int, dict[str, dict[str, Any]]],
    time_seconds: float,
) -> list[list[float]]:
    locals_out = list(base_locals)
    for node_index, node_tracks in tracks.items():
        node = document["nodes"][node_index]
        translation = list(map(float, node.get("translation", [0.0, 0.0, 0.0])))
        rotation = list(map(float, node.get("rotation", [0.0, 0.0, 0.0, 1.0])))
        scale = list(map(float, node.get("scale", [1.0, 1.0, 1.0])))
        if "translation" in node_tracks:
            translation = list(evaluate_track(node_tracks["translation"], time_seconds))
        if "rotation" in node_tracks:
            rotation = normalize_quat(evaluate_track(node_tracks["rotation"], time_seconds))
        if "scale" in node_tracks:
            scale = list(evaluate_track(node_tracks["scale"], time_seconds))
        locals_out[node_index] = trs_matrix(translation, rotation, scale)
    return locals_out


def world_matrices_for_nodes(
    local_matrices: list[list[float]],
    parents: list[int | None],
) -> list[list[float]]:
    cache: list[list[float] | None] = [None for _ in local_matrices]

    def resolve(node_index: int) -> list[float]:
        cached = cache[node_index]
        if cached is not None:
            return cached
        parent_index = parents[node_index]
        if parent_index is None:
            result = local_matrices[node_index]
        else:
            result = mat4_mul(resolve(parent_index), local_matrices[node_index])
        cache[node_index] = result
        return result

    return [resolve(index) for index in range(len(local_matrices))]


def evaluate_track(track: dict[str, Any], time_seconds: float) -> tuple[float, ...]:
    times: list[float] = track["times"]
    values: list[tuple[float, ...]] = track["values"]
    if time_seconds <= times[0]:
        return values[0]
    if time_seconds >= times[-1]:
        return values[-1]
    upper = bisect.bisect_right(times, time_seconds)
    lower = max(0, upper - 1)
    upper = min(upper, len(times) - 1)
    start_time = times[lower]
    end_time = times[upper]
    if end_time <= start_time or track.get("interpolation") == "STEP":
        return values[lower]
    t = (time_seconds - start_time) / (end_time - start_time)
    start = values[lower]
    end = values[upper]
    if len(start) == 4:
        return tuple(nlerp_quat(start, end, t))
    return tuple(start[index] + (end[index] - start[index]) * t for index in range(len(start)))


def skin_positions(
    positions: list[tuple[float, float, float]],
    joints: list[tuple[Any, ...]],
    weights: list[tuple[Any, ...]],
    skin_matrices: list[list[float]],
) -> list[dict[str, float]]:
    out: list[dict[str, float]] = []
    for position, joint_tuple, weight_tuple in zip(positions, joints, weights):
        skinned = [0.0, 0.0, 0.0]
        total_weight = 0.0
        for channel in range(min(len(joint_tuple), len(weight_tuple))):
            weight = float(weight_tuple[channel])
            if weight <= 0.0:
                continue
            joint_index = int(joint_tuple[channel])
            if joint_index < 0 or joint_index >= len(skin_matrices):
                raise SmokeError(f"vertex references joint index out of range: {joint_index}")
            transformed = transform_point(skin_matrices[joint_index], position)
            skinned[0] += transformed[0] * weight
            skinned[1] += transformed[1] * weight
            skinned[2] += transformed[2] * weight
            total_weight += weight
        if total_weight <= 0.0:
            skinned = [position[0], position[1], position[2]]
        elif abs(total_weight - 1.0) > 1.0e-5:
            skinned = [component / total_weight for component in skinned]
        out.append(vec3_dict(skinned))
    return out


def node_local_matrix(node: dict[str, Any]) -> list[float]:
    if "matrix" in node:
        return gltf_mat4_to_row_major(tuple(map(float, node["matrix"])))
    return trs_matrix(
        list(map(float, node.get("translation", [0.0, 0.0, 0.0]))),
        list(map(float, node.get("rotation", [0.0, 0.0, 0.0, 1.0]))),
        list(map(float, node.get("scale", [1.0, 1.0, 1.0]))),
    )


def trs_matrix(translation: list[float], rotation: list[float], scale: list[float]) -> list[float]:
    x, y, z, w = normalize_quat(rotation)
    xx = x * x
    yy = y * y
    zz = z * z
    xy = x * y
    xz = x * z
    yz = y * z
    wx = w * x
    wy = w * y
    wz = w * z
    sx, sy, sz = scale
    return [
        (1.0 - 2.0 * (yy + zz)) * sx,
        (2.0 * (xy - wz)) * sy,
        (2.0 * (xz + wy)) * sz,
        translation[0],
        (2.0 * (xy + wz)) * sx,
        (1.0 - 2.0 * (xx + zz)) * sy,
        (2.0 * (yz - wx)) * sz,
        translation[1],
        (2.0 * (xz - wy)) * sx,
        (2.0 * (yz + wx)) * sy,
        (1.0 - 2.0 * (xx + yy)) * sz,
        translation[2],
        0.0,
        0.0,
        0.0,
        1.0,
    ]


def identity_mat4() -> list[float]:
    return [
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]


def gltf_mat4_to_row_major(values: tuple[Any, ...]) -> list[float]:
    if len(values) != 16:
        raise SmokeError("MAT4 accessor value must contain 16 components")
    return [float(values[column * 4 + row]) for row in range(4) for column in range(4)]


def mat4_mul(left: list[float], right: list[float]) -> list[float]:
    out = [0.0 for _ in range(16)]
    for row in range(4):
        for column in range(4):
            out[row * 4 + column] = sum(
                left[row * 4 + k] * right[k * 4 + column] for k in range(4)
            )
    return out


def transform_point(matrix: list[float], point: tuple[float, float, float]) -> tuple[float, float, float]:
    x, y, z = point
    return (
        matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3],
        matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7],
        matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11],
    )


def invert_affine(matrix: list[float]) -> list[float]:
    a00, a01, a02 = matrix[0], matrix[1], matrix[2]
    a10, a11, a12 = matrix[4], matrix[5], matrix[6]
    a20, a21, a22 = matrix[8], matrix[9], matrix[10]
    det = (
        a00 * (a11 * a22 - a12 * a21)
        - a01 * (a10 * a22 - a12 * a20)
        + a02 * (a10 * a21 - a11 * a20)
    )
    if abs(det) <= 1.0e-12:
        raise SmokeError("cannot invert singular affine matrix")
    inv_det = 1.0 / det
    r00 = (a11 * a22 - a12 * a21) * inv_det
    r01 = (a02 * a21 - a01 * a22) * inv_det
    r02 = (a01 * a12 - a02 * a11) * inv_det
    r10 = (a12 * a20 - a10 * a22) * inv_det
    r11 = (a00 * a22 - a02 * a20) * inv_det
    r12 = (a02 * a10 - a00 * a12) * inv_det
    r20 = (a10 * a21 - a11 * a20) * inv_det
    r21 = (a01 * a20 - a00 * a21) * inv_det
    r22 = (a00 * a11 - a01 * a10) * inv_det
    tx, ty, tz = matrix[3], matrix[7], matrix[11]
    return [
        r00,
        r01,
        r02,
        -(r00 * tx + r01 * ty + r02 * tz),
        r10,
        r11,
        r12,
        -(r10 * tx + r11 * ty + r12 * tz),
        r20,
        r21,
        r22,
        -(r20 * tx + r21 * ty + r22 * tz),
        0.0,
        0.0,
        0.0,
        1.0,
    ]


def normalize_quat(value: tuple[float, ...] | list[float]) -> list[float]:
    x, y, z, w = float(value[0]), float(value[1]), float(value[2]), float(value[3])
    length = math.sqrt(x * x + y * y + z * z + w * w)
    if length <= 1.0e-12:
        return [0.0, 0.0, 0.0, 1.0]
    return [x / length, y / length, z / length, w / length]


def nlerp_quat(
    start: tuple[float, ...],
    end: tuple[float, ...],
    t: float,
) -> list[float]:
    sx, sy, sz, sw = normalize_quat(start)
    ex, ey, ez, ew = normalize_quat(end)
    if sx * ex + sy * ey + sz * ez + sw * ew < 0.0:
        ex, ey, ez, ew = -ex, -ey, -ez, -ew
    return normalize_quat(
        [
            sx + (ex - sx) * t,
            sy + (ey - sy) * t,
            sz + (ez - sz) * t,
            sw + (ew - sw) * t,
        ]
    )


def merge_bounds(
    current_min: dict[str, float] | None,
    current_max: dict[str, float] | None,
    next_min: dict[str, float],
    next_max: dict[str, float],
) -> tuple[dict[str, float], dict[str, float]]:
    if current_min is None or current_max is None:
        return dict(next_min), dict(next_max)
    return (
        {axis: min(current_min[axis], next_min[axis]) for axis in ("x", "y", "z")},
        {axis: max(current_max[axis], next_max[axis]) for axis in ("x", "y", "z")},
    )


def validate_sequence(payload: dict[str, Any]) -> None:
    if payload["schema_id"] != SEQUENCE_SCHEMA_ID:
        raise SmokeError("sequence schema mismatch")
    if payload["frame_count"] != len(payload["frames"]):
        raise SmokeError("sequence frame_count does not match frames length")
    if not payload["triangles"]:
        raise SmokeError("sequence contains no triangles")
    vertex_count = payload["vertex_count"]
    for triangle in payload["triangles"]:
        if len(triangle) != 3 or any(index < 0 or index >= vertex_count for index in triangle):
            raise SmokeError(f"triangle index out of range: {triangle}")
    for frame in payload["frames"]:
        if len(frame["positions"]) != vertex_count:
            raise SmokeError("frame position count does not match vertex_count")
        for position in frame["positions"]:
            if not all(math.isfinite(position[axis]) for axis in ("x", "y", "z")):
                raise SmokeError("frame contains non-finite position")


def vec3_dict(values: list[float] | tuple[float, float, float]) -> dict[str, float]:
    return {
        "x": round_float(values[0]),
        "y": round_float(values[1]),
        "z": round_float(values[2]),
    }


def round_float(value: float) -> float:
    return round(float(value), 9)


if __name__ == "__main__":
    sys.exit(main())
