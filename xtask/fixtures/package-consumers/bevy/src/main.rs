use bevy_app::{App, FixedUpdate};
use bevy_boxdd::boxdd::{Aabb, Position, QueryFilter};
use bevy_boxdd::prelude::{BoxddPhysicsContext, BoxddPhysicsPlugin, BoxddPhysicsSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "double-precision")]
    assert_eq!(
        std::mem::size_of::<bevy_boxdd::boxdd::WorldScalar>(),
        8
    );
    #[cfg(not(feature = "double-precision"))]
    assert_eq!(
        std::mem::size_of::<bevy_boxdd::boxdd::WorldScalar>(),
        4
    );

    let foundation = bevy_boxdd::boxdd::Foundation::initialize_default()?;
    let mut app = App::new();
    app.add_plugins(BoxddPhysicsPlugin::new(
        foundation,
        BoxddPhysicsSettings::default(),
    ));
    app.world_mut().run_schedule(FixedUpdate);

    let context = app.world().non_send::<BoxddPhysicsContext>();
    let world = context.world().expect("plugin must install a live world");
    let query = world.query()?;
    let hits = query.overlap_aabb(
        Position::ZERO,
        Aabb::new([-1.0, -1.0], [1.0, 1.0])?,
        QueryFilter::default(),
    )?;
    assert!(hits.is_empty());
    drop(query);
    drop(app);
    Ok(())
}
