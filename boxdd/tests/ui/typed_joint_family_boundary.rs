use boxdd::{DistanceJointDef, Foundation};

fn main() {
    let foundation = Foundation::initialize_default().unwrap();
    let mut world = foundation
        .create_world(foundation.world_def())
        .unwrap();
    let body_a = world
        .create_body(world.body_builder().build().unwrap())
        .unwrap();
    let body_b = world
        .create_body(world.body_builder().build().unwrap())
        .unwrap();
    let definition = DistanceJointDef::new(world.joint_base(body_a, body_b).unwrap());
    let joint_id = world.create_distance_joint(&definition).unwrap();
    let mut distance = world.joint(joint_id).unwrap().into_distance().unwrap();
    let _ = distance.set_target_angle(0.25);
}
