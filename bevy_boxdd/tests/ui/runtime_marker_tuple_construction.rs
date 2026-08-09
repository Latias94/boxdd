use bevy_boxdd::{
    BoxddBody, BoxddJoint, BoxddShape,
    boxdd::{BodyId, JointId, ShapeId},
};

fn forge_body(id: BodyId) {
    let _ = BoxddBody(id);
}

fn forge_shape(id: ShapeId) {
    let _ = BoxddShape(id);
}

fn forge_joint(id: JointId) {
    let _ = BoxddJoint(id);
}

fn main() {}
