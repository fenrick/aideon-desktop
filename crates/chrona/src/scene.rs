//! Scene generation utilities for the Canvas MVP.

use aideon_praxis::canvas::CanvasShape;

/// Generate a small synthetic scene of shapes for the renderer to display.
/// Stable and deterministic so the UI can test against it.
pub fn generate_demo_scene() -> Vec<CanvasShape> {
    vec![
        CanvasShape {
            id: "s1".into(),
            type_id: "rect".into(),
            x: 200.0,
            y: 200.0,
            w: 200.0,
            h: 120.0,
            label: Some("Node A".into()),
        },
        CanvasShape {
            id: "s2".into(),
            type_id: "rect".into(),
            x: 600.0,
            y: 480.0,
            w: 220.0,
            h: 140.0,
            label: Some("Node B".into()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::generate_demo_scene;

    #[test]
    fn returns_two_rects() {
        let s = generate_demo_scene();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].type_id, "rect");
    }
}
