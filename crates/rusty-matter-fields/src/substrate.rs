use rusty_matter_mesh::{
    MeshSurfaceSample, MeshSurfaceSampleSet, MeshSurfaceTopologyKey,
    MESH_SURFACE_TOPOLOGY_KEY_SCHEMA_ID,
};
use rusty_matter_model::Vec3;

use crate::{MatterFieldError, SURFACE_FIELD_NODE_SCHEMA_ID, SURFACE_FIELD_SUBSTRATE_SCHEMA_ID};

/// One stable surface-field node derived from a mesh surface sample.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldNode {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable node identifier.
    pub node_id: String,
    /// Source mesh sample identifier.
    pub sample_id: String,
    /// Node index within the substrate.
    pub node_index: usize,
    /// Evaluated sample position.
    pub position: Vec3,
    /// Evaluated sample normal.
    pub normal: Vec3,
    /// Source triangle index.
    pub triangle_index: usize,
    /// Barycentric coordinates inside the source triangle.
    ///
    /// These coordinates keep the node anchored to the source mesh even when a
    /// later runtime deforms or swaps vertex positions without changing
    /// topology.
    pub barycentric: [f32; 3],
    /// Same-surface first-tier neighbor nodes.
    pub first_tier_neighbors: Vec<usize>,
    /// Same-surface second-tier neighbor nodes.
    pub second_tier_neighbors: Vec<usize>,
}

impl SurfaceFieldNode {
    fn from_sample(
        substrate_id: &str,
        node_index: usize,
        sample: &MeshSurfaceSample,
        first_tier_neighbors: Vec<usize>,
        second_tier_neighbors: Vec<usize>,
    ) -> Self {
        Self {
            schema_id: SURFACE_FIELD_NODE_SCHEMA_ID.to_owned(),
            node_id: format!("{substrate_id}.node.{node_index:04}"),
            sample_id: sample.sample_id.clone(),
            node_index,
            position: sample.position,
            normal: sample.normal,
            triangle_index: sample.triangle_index,
            barycentric: sample.barycentric,
            first_tier_neighbors,
            second_tier_neighbors,
        }
    }

    /// Validates the node against a substrate node count.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, vectors, or neighbor targets
    /// are invalid.
    pub fn validate(&self, node_count: usize) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_NODE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_NODE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.node_id.trim().is_empty() {
            return Err(MatterFieldError::EmptyNodeId);
        }
        if self.sample_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "node sample id must not be empty",
            ));
        }
        if self.node_index >= node_count {
            return Err(MatterFieldError::InvalidSubstrate(
                "node index must be inside substrate node count",
            ));
        }
        if !self.position.is_finite() || !self.normal.is_finite() {
            return Err(MatterFieldError::InvalidSubstrate(
                "node position and normal must be finite",
            ));
        }
        if self.normal.length_squared() <= 1.0e-10 {
            return Err(MatterFieldError::InvalidSubstrate(
                "node normal must be non-zero",
            ));
        }
        if !self
            .barycentric
            .iter()
            .all(|value| value.is_finite() && *value >= -1.0e-5 && *value <= 1.0 + 1.0e-5)
            || (self.barycentric[0] + self.barycentric[1] + self.barycentric[2] - 1.0).abs()
                > 1.0e-4
        {
            return Err(MatterFieldError::InvalidSubstrate(
                "node barycentric anchor must be finite and normalized",
            ));
        }
        validate_neighbors(self.node_index, node_count, &self.first_tier_neighbors)?;
        validate_neighbors(self.node_index, node_count, &self.second_tier_neighbors)?;
        Ok(())
    }
}

/// Surface-field graph substrate over mesh sample nodes and neighbor tiers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFieldSubstrate {
    /// Schema identifier.
    pub schema_id: String,
    /// Stable substrate identifier.
    pub substrate_id: String,
    /// Source sample set identifier.
    pub sample_set_id: String,
    /// Source surface identifier.
    pub surface_id: String,
    /// Source mesh topology key.
    pub topology_key: MeshSurfaceTopologyKey,
    /// Stable nodes derived from the source sample set.
    pub nodes: Vec<SurfaceFieldNode>,
}

impl SurfaceFieldSubstrate {
    /// Builds a field substrate from a mesh surface sample set.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when the sample set or generated substrate
    /// is invalid.
    pub fn from_sample_set(
        substrate_id: impl Into<String>,
        samples: &MeshSurfaceSampleSet,
    ) -> Result<Self, MatterFieldError> {
        if !samples.is_valid() {
            return Err(MatterFieldError::InvalidSubstrate(
                "source mesh sample set must be valid",
            ));
        }
        let substrate_id = substrate_id.into();
        let nodes = samples
            .samples
            .iter()
            .enumerate()
            .map(|(node_index, sample)| {
                SurfaceFieldNode::from_sample(
                    &substrate_id,
                    node_index,
                    sample,
                    samples.first_tier_neighbors[node_index].clone(),
                    samples.second_tier_neighbors[node_index].clone(),
                )
            })
            .collect::<Vec<_>>();
        let substrate = Self {
            schema_id: SURFACE_FIELD_SUBSTRATE_SCHEMA_ID.to_owned(),
            substrate_id,
            sample_set_id: samples.sample_set_id.clone(),
            surface_id: samples.surface_id.clone(),
            topology_key: samples.topology_key.clone(),
            nodes,
        };
        substrate.validate()?;
        Ok(substrate)
    }

    /// Returns the substrate node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the substrate has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns first-tier directed edge count.
    #[must_use]
    pub fn first_tier_edge_count(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.first_tier_neighbors.len())
            .sum()
    }

    /// Returns second-tier directed edge count.
    #[must_use]
    pub fn second_tier_edge_count(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.second_tier_neighbors.len())
            .sum()
    }

    /// Returns first-tier neighbors for a node.
    #[must_use]
    pub fn first_tier_neighbors(&self, node_index: usize) -> Option<&[usize]> {
        self.nodes
            .get(node_index)
            .map(|node| node.first_tier_neighbors.as_slice())
    }

    /// Returns second-tier neighbors for a node.
    #[must_use]
    pub fn second_tier_neighbors(&self, node_index: usize) -> Option<&[usize]> {
        self.nodes
            .get(node_index)
            .map(|node| node.second_tier_neighbors.as_slice())
    }

    /// Validates the substrate contract.
    ///
    /// # Errors
    ///
    /// Returns [`MatterFieldError`] when metadata, nodes, or neighbor tiers are
    /// invalid.
    pub fn validate(&self) -> Result<(), MatterFieldError> {
        if self.schema_id != SURFACE_FIELD_SUBSTRATE_SCHEMA_ID {
            return Err(MatterFieldError::UnexpectedSchema {
                expected: SURFACE_FIELD_SUBSTRATE_SCHEMA_ID,
                actual: self.schema_id.clone(),
            });
        }
        if self.substrate_id.trim().is_empty() {
            return Err(MatterFieldError::EmptySubstrateId);
        }
        if self.sample_set_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "sample set id must not be empty",
            ));
        }
        if self.surface_id.trim().is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "surface id must not be empty",
            ));
        }
        if self.topology_key.schema_id != MESH_SURFACE_TOPOLOGY_KEY_SCHEMA_ID {
            return Err(MatterFieldError::InvalidSubstrate(
                "topology key schema must match mesh surface topology key",
            ));
        }
        if self.topology_key.vertex_count == 0 || self.topology_key.triangle_count == 0 {
            return Err(MatterFieldError::InvalidSubstrate(
                "topology key must describe a non-empty surface",
            ));
        }
        if self.nodes.is_empty() {
            return Err(MatterFieldError::InvalidSubstrate(
                "substrate must contain nodes",
            ));
        }
        let node_count = self.nodes.len();
        for (expected_index, node) in self.nodes.iter().enumerate() {
            if node.node_index != expected_index {
                return Err(MatterFieldError::InvalidSubstrate(
                    "node indices must match node order",
                ));
            }
            node.validate(node_count)?;
        }
        Ok(())
    }
}

fn validate_neighbors(
    node_index: usize,
    node_count: usize,
    neighbors: &[usize],
) -> Result<(), MatterFieldError> {
    let mut seen = Vec::with_capacity(neighbors.len());
    for &neighbor_index in neighbors {
        if neighbor_index >= node_count {
            return Err(MatterFieldError::InvalidNeighbor {
                node_index,
                neighbor_index,
            });
        }
        if neighbor_index == node_index {
            return Err(MatterFieldError::SelfNeighbor { node_index });
        }
        if seen.contains(&neighbor_index) {
            return Err(MatterFieldError::DuplicateNeighbor {
                node_index,
                neighbor_index,
            });
        }
        seen.push(neighbor_index);
    }
    Ok(())
}
