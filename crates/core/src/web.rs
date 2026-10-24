//! What the page needs to know once: every size, stride and offset, the
//! passes, and the light's fixed weights. The page holds none of these
//! numbers itself (Builder D3). After Mage Tower Battle's web.rs.

use serde_json::json;

use crate::fx::ONE;
use crate::mesh::{self, Mesh, VERTEX_VALUES};
use crate::render::{COMMAND_VALUES, INSTANCE_VALUES, VIEW_UNIFORMS, VIEW_VALUES};

/// The light's weights (sky and ground light, sun on the faces turned to
/// it, sun wrapped round the rest, and the wrap's half), the glow pass's
/// strength and a glowing solid's brightness, the ink outline's width in
/// metres and its colour (Protection of the Spheres' violet ink).
/// (name, size, values in Q16.)
const UNIFORMS: [(&str, usize, [i32; 4]); 5] = [
    ("uEps", 1, [7, 0, 0, 0]),
    (
        "uMix",
        4,
        [ONE * 62 / 100, ONE * 48 / 100, ONE * 26 / 100, ONE / 2],
    ),
    ("uGlow", 2, [ONE * 16 / 10, ONE * 108 / 100, 0, 0]),
    ("uInk", 1, [ONE * 3 / 100, 0, 0, 0]),
    (
        "uInkColour",
        3,
        [ONE * 43 / 255, ONE * 33 / 255, ONE * 64 / 255, 0],
    ),
];

/// The uniforms' values in order, Q16; the shim turns them into floats.
pub fn uniform_values() -> Vec<i32> {
    UNIFORMS.iter().flat_map(|(_, n, v)| v[..*n].to_vec()).collect()
}

pub fn numbers() -> String {
    let meshes: Vec<serde_json::Value> = Mesh::ALL
        .iter()
        .map(|&m| json!({"id": m as u32, "vertices": mesh::vertices(m).len() / VERTEX_VALUES}))
        .collect();
    json!({
        "meshes": meshes,
        "vertex": {"stride": VERTEX_VALUES * 4, "attributes": [["aPos", 3, 0], ["aNormal", 3, 12]]},
        "instance": {"stride": INSTANCE_VALUES * 4, "attributes": [["iA", 4, 0], ["iB", 4, 16], ["iC", 4, 32], ["iD", 4, 48]]},
        "command_values": COMMAND_VALUES,
        "ops": {"viewport": 0, "draw": 1},
        "view": {"size": VIEW_VALUES, "matrix": [0, 16], "clear": [20, 24], "uniforms": VIEW_UNIFORMS.iter().map(|(name, at, n)| json!([name, at, n])).collect::<Vec<_>>()},
        "passes": {"solid": 0, "glow": 1, "ink": 2},
        "buffers": {"static": 0, "dynamic": 1},
        "uniforms": UNIFORMS.iter().map(|(name, n, _)| json!([name, n])).collect::<Vec<_>>(),
    })
    .to_string()
}

pub fn mesh_values(id: u32) -> Vec<i32> {
    Mesh::ALL
        .iter()
        .find(|&&m| m as u32 == id)
        .map(|&m| mesh::vertices(m))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_name_every_mesh_and_every_uniform_value() {
        let n: serde_json::Value = serde_json::from_str(&numbers()).unwrap();
        assert_eq!(n["meshes"].as_array().unwrap().len(), Mesh::ALL.len());
        assert_eq!(n["passes"]["ink"], 2);
        let sizes: usize = UNIFORMS.iter().map(|u| u.1).sum();
        assert_eq!(uniform_values().len(), sizes);
        assert_eq!(mesh_values(Mesh::Cube as u32).len(), 36 * VERTEX_VALUES);
    }
}
