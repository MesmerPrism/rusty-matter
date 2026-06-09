"""Extract glTF binary triangle primitives as Matter mesh surfaces.

This tool is intentionally dependency-free and lives outside Matter core
crates. Platform adapters may produce GLB files, but Matter's reusable mesh
contracts should only see generic triangle surfaces.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import struct
import sys
from pathlib import Path
from typing import Any


GLB_MAGIC = 0x46546C67
GLB_VERSION = 2
JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942
TRIANGLES_MODE = 4
TRIANGLE_MESH_SURFACE_SCHEMA_ID = "rusty.matter.mesh.surface.v1"
SUMMARY_SCHEMA_ID = "rusty.matter.tools.glb_mesh_surface_smoke.v1"
FNV_OFFSET_BASIS = 0xCBF29CE484222325
FNV_PRIME = 0x00000100000001B3

COMPONENT_FORMATS = {
    5120: ("b", 1),
    5121: ("B", 1),
    5122: ("h", 2),
    5123: ("H", 2),
    5125: ("I", 4),
    5126: ("f", 4),
}

TYPE_COMPONENTS = {
    "SCALAR": 1,
    "VEC2": 2,
    "VEC3": 3,
    "VEC4": 4,
    "MAT2": 4,
    "MAT3": 9,
    "MAT4": 16,
}


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Extract GLB mesh primitives into Matter TriangleMeshSurface JSON files."
    )
    parser.add_argument("--glb", required=True, type=Path, help="Input .glb file")
    parser.add_argument(
        "--output-root",
        required=True,
        type=Path,
        help="Directory for summary.json and surfaces/*.surface.json",
    )
    args = parser.parse_args()

    try:
        result = extract(args.glb, args.output_root)
    except SmokeError as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        return 1

    print(
        "[PASS] extracted "
        f"{result['surface_count']} surfaces, "
        f"{result['total_vertex_count']} vertices, "
        f"{result['total_triangle_count']} triangles"
    )
    print(f"[WRITE] {result['summary_path']}")
    return 0


class SmokeError(RuntimeError):
    """User-facing extraction error."""


def extract(glb_path: Path, output_root: Path) -> dict[str, Any]:
    glb_path = glb_path.resolve()
    if not glb_path.exists():
        raise SmokeError(f"input GLB does not exist: {glb_path}")
    if glb_path.suffix.lower() != ".glb":
        raise SmokeError(f"input must be a .glb file: {glb_path}")

    data = glb_path.read_bytes()
    document, bin_chunk = read_glb(data)
    output_root.mkdir(parents=True, exist_ok=True)
    surfaces_dir = output_root / "surfaces"
    surfaces_dir.mkdir(parents=True, exist_ok=True)

    surfaces: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    node_names_by_mesh = source_node_names_by_mesh(document)

    for mesh_index, mesh in enumerate(document.get("meshes", [])):
        mesh_name = mesh.get("name") or f"mesh_{mesh_index:02d}"
        mesh_token = safe_token(mesh_name)
        primitives = mesh.get("primitives", [])
        for primitive_index, primitive in enumerate(primitives):
            mode = primitive.get("mode", TRIANGLES_MODE)
            if mode != TRIANGLES_MODE:
                skipped.append(
                    {
                        "mesh_index": mesh_index,
                        "primitive_index": primitive_index,
                        "reason": f"unsupported primitive mode {mode}",
                    }
                )
                continue
            attributes = primitive.get("attributes", {})
            position_accessor = attributes.get("POSITION")
            if position_accessor is None:
                skipped.append(
                    {
                        "mesh_index": mesh_index,
                        "primitive_index": primitive_index,
                        "reason": "missing POSITION attribute",
                    }
                )
                continue

            positions = read_positions(document, bin_chunk, position_accessor)
            triangles = read_triangles(document, bin_chunk, primitive, len(positions))
            surface_id = (
                f"mesh.glb_surface.{mesh_index:02d}.{primitive_index:02d}.{mesh_token}"
            )
            surface = {
                "schema_id": TRIANGLE_MESH_SURFACE_SCHEMA_ID,
                "surface_id": surface_id,
                "positions": positions,
                "triangles": triangles,
            }
            validate_surface(surface)

            file_name = f"{mesh_index:02d}_{primitive_index:02d}_{mesh_token}.surface.json"
            surface_path = surfaces_dir / file_name
            write_json(surface_path, surface)
            bounds_min, bounds_max = bounds_for_positions(positions)
            surface_record = {
                "surface_id": surface_id,
                "surface_path": str(surface_path.relative_to(output_root)).replace("\\", "/"),
                "mesh_index": mesh_index,
                "mesh_name": mesh_name,
                "primitive_index": primitive_index,
                "source_node_names": node_names_by_mesh.get(mesh_index, []),
                "attributes": sorted(attributes.keys()),
                "vertex_count": len(positions),
                "triangle_count": len(triangles),
                "bounds_min": bounds_min,
                "bounds_max": bounds_max,
                "topology_index_hash": mesh_surface_index_hash(triangles),
            }
            surfaces.append(surface_record)

    if not surfaces:
        raise SmokeError("no supported triangle mesh primitives were extracted")

    summary = {
        "schema_id": SUMMARY_SCHEMA_ID,
        "source_file_name": glb_path.name,
        "source_size_bytes": len(data),
        "source_sha256": hashlib.sha256(data).hexdigest(),
        "asset": document.get("asset", {}),
        "scene_count": len(document.get("scenes", [])),
        "node_count": len(document.get("nodes", [])),
        "mesh_count": len(document.get("meshes", [])),
        "skin_count": len(document.get("skins", [])),
        "animation_count": len(document.get("animations", [])),
        "animation_channel_count": sum(
            len(animation.get("channels", []))
            for animation in document.get("animations", [])
        ),
        "surface_count": len(surfaces),
        "total_vertex_count": sum(surface["vertex_count"] for surface in surfaces),
        "total_triangle_count": sum(surface["triangle_count"] for surface in surfaces),
        "surfaces": surfaces,
        "skipped_primitives": skipped,
    }
    summary_path = output_root / "summary.json"
    write_json(summary_path, summary)

    return {
        "surface_count": summary["surface_count"],
        "total_vertex_count": summary["total_vertex_count"],
        "total_triangle_count": summary["total_triangle_count"],
        "summary_path": summary_path,
    }


def read_glb(data: bytes) -> tuple[dict[str, Any], bytes]:
    if len(data) < 12:
        raise SmokeError("GLB is too short")
    magic, version, declared_length = struct.unpack_from("<III", data, 0)
    if magic != GLB_MAGIC:
        raise SmokeError("input is not a GLB file")
    if version != GLB_VERSION:
        raise SmokeError(f"unsupported GLB version {version}")
    if declared_length != len(data):
        raise SmokeError(
            f"GLB length mismatch: header={declared_length} actual={len(data)}"
        )

    json_chunk = None
    bin_chunk = None
    offset = 12
    while offset < len(data):
        if offset + 8 > len(data):
            raise SmokeError("truncated GLB chunk header")
        chunk_length, chunk_type = struct.unpack_from("<II", data, offset)
        offset += 8
        chunk_end = offset + chunk_length
        if chunk_end > len(data):
            raise SmokeError("truncated GLB chunk data")
        chunk = data[offset:chunk_end]
        offset = chunk_end
        if chunk_type == JSON_CHUNK:
            json_chunk = chunk
        elif chunk_type == BIN_CHUNK:
            bin_chunk = chunk

    if json_chunk is None:
        raise SmokeError("GLB is missing a JSON chunk")
    if bin_chunk is None:
        raise SmokeError("GLB is missing a BIN chunk")

    try:
        document = json.loads(json_chunk.decode("utf-8").rstrip(" \t\r\n\0"))
    except json.JSONDecodeError as error:
        raise SmokeError(f"invalid GLB JSON chunk: {error}") from error
    return document, bin_chunk


def source_node_names_by_mesh(document: dict[str, Any]) -> dict[int, list[str]]:
    result: dict[int, list[str]] = {}
    for node_index, node in enumerate(document.get("nodes", [])):
        mesh_index = node.get("mesh")
        if mesh_index is None:
            continue
        name = node.get("name") or f"node_{node_index:02d}"
        result.setdefault(mesh_index, []).append(name)
    return result


def read_positions(
    document: dict[str, Any], bin_chunk: bytes, accessor_index: int
) -> list[dict[str, float]]:
    accessor = accessor_at(document, accessor_index)
    if accessor.get("componentType") != 5126 or accessor.get("type") != "VEC3":
        raise SmokeError(
            f"POSITION accessor {accessor_index} must be componentType 5126 VEC3"
        )
    values = read_accessor(document, bin_chunk, accessor_index)
    positions = [{"x": x, "y": y, "z": z} for x, y, z in values]
    if not positions:
        raise SmokeError(f"POSITION accessor {accessor_index} is empty")
    return positions


def read_triangles(
    document: dict[str, Any],
    bin_chunk: bytes,
    primitive: dict[str, Any],
    vertex_count: int,
) -> list[list[int]]:
    indices_accessor = primitive.get("indices")
    if indices_accessor is None:
        if vertex_count % 3 != 0:
            raise SmokeError("unindexed triangle primitive vertex count is not divisible by 3")
        indices = list(range(vertex_count))
    else:
        accessor = accessor_at(document, indices_accessor)
        if accessor.get("type") != "SCALAR" or accessor.get("componentType") not in {
            5121,
            5123,
            5125,
        }:
            raise SmokeError(
                f"indices accessor {indices_accessor} must be unsigned SCALAR data"
            )
        indices = [value[0] for value in read_accessor(document, bin_chunk, indices_accessor)]

    if len(indices) % 3 != 0:
        raise SmokeError("triangle index count is not divisible by 3")
    triangles: list[list[int]] = []
    for offset in range(0, len(indices), 3):
        triangle = [int(indices[offset]), int(indices[offset + 1]), int(indices[offset + 2])]
        if any(index < 0 or index >= vertex_count for index in triangle):
            raise SmokeError(f"triangle index out of range: {triangle}")
        if triangle[0] == triangle[1] or triangle[1] == triangle[2] or triangle[0] == triangle[2]:
            raise SmokeError(f"degenerate triangle: {triangle}")
        triangles.append(triangle)
    if not triangles:
        raise SmokeError("primitive contains no triangles")
    return triangles


def accessor_at(document: dict[str, Any], accessor_index: int) -> dict[str, Any]:
    accessors = document.get("accessors", [])
    if accessor_index < 0 or accessor_index >= len(accessors):
        raise SmokeError(f"accessor index out of range: {accessor_index}")
    return accessors[accessor_index]


def read_accessor(
    document: dict[str, Any], bin_chunk: bytes, accessor_index: int
) -> list[tuple[Any, ...]]:
    accessor = accessor_at(document, accessor_index)
    buffer_view_index = accessor.get("bufferView")
    if buffer_view_index is None:
        raise SmokeError(f"accessor {accessor_index} has no bufferView")
    buffer_views = document.get("bufferViews", [])
    if buffer_view_index < 0 or buffer_view_index >= len(buffer_views):
        raise SmokeError(f"bufferView index out of range: {buffer_view_index}")
    buffer_view = buffer_views[buffer_view_index]
    if buffer_view.get("buffer", 0) != 0:
        raise SmokeError("only single-buffer GLB files are supported")

    component_type = accessor.get("componentType")
    accessor_type = accessor.get("type")
    count = accessor.get("count")
    if component_type not in COMPONENT_FORMATS:
        raise SmokeError(f"unsupported accessor component type {component_type}")
    if accessor_type not in TYPE_COMPONENTS:
        raise SmokeError(f"unsupported accessor type {accessor_type}")
    if not isinstance(count, int) or count < 0:
        raise SmokeError(f"invalid accessor count {count}")

    format_char, component_size = COMPONENT_FORMATS[component_type]
    component_count = TYPE_COMPONENTS[accessor_type]
    element_size = component_size * component_count
    stride = buffer_view.get("byteStride", element_size)
    if stride < element_size:
        raise SmokeError(f"bufferView {buffer_view_index} stride is too small")

    view_offset = buffer_view.get("byteOffset", 0)
    accessor_offset = accessor.get("byteOffset", 0)
    base_offset = view_offset + accessor_offset
    view_length = buffer_view.get("byteLength", 0)
    if base_offset < 0 or view_length < 0:
        raise SmokeError("negative GLB byte offsets are invalid")
    if view_offset + view_length > len(bin_chunk):
        raise SmokeError(f"bufferView {buffer_view_index} extends beyond BIN chunk")
    if count and base_offset + (count - 1) * stride + element_size > view_offset + view_length:
        raise SmokeError(f"accessor {accessor_index} extends beyond bufferView")

    unpack_format = "<" + format_char * component_count
    values: list[tuple[Any, ...]] = []
    for element_index in range(count):
        start = base_offset + element_index * stride
        values.append(struct.unpack_from(unpack_format, bin_chunk, start))
    return values


def validate_surface(surface: dict[str, Any]) -> None:
    for index, position in enumerate(surface["positions"]):
        if not all(math.isfinite(position[axis]) for axis in ("x", "y", "z")):
            raise SmokeError(f"non-finite position at index {index}")


def bounds_for_positions(
    positions: list[dict[str, float]]
) -> tuple[dict[str, float], dict[str, float]]:
    min_position = dict(positions[0])
    max_position = dict(positions[0])
    for position in positions[1:]:
        for axis in ("x", "y", "z"):
            min_position[axis] = min(min_position[axis], position[axis])
            max_position[axis] = max(max_position[axis], position[axis])
    return min_position, max_position


def mesh_surface_index_hash(triangles: list[list[int]]) -> int:
    hash_value = FNV_OFFSET_BASIS
    for triangle in triangles:
        for index in triangle:
            for byte in int(index).to_bytes(4, byteorder="little", signed=False):
                hash_value ^= byte
                hash_value = (hash_value * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return hash_value


def safe_token(value: str) -> str:
    token = re.sub(r"[^A-Za-z0-9_.-]+", "_", value.strip()).strip("._-")
    return token.lower() or "mesh"


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(payload, indent=2, sort_keys=False)
    path.write_text(text + "\n", encoding="utf-8")


if __name__ == "__main__":
    sys.exit(main())
