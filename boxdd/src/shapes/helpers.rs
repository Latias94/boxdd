use boxdd_sys::ffi;

/// Capsule helper
pub fn capsule<V: Into<crate::types::Vec2>>(c1: V, c2: V, radius: f32) -> ffi::b2Capsule {
    ffi::b2Capsule {
        center1: ffi::b2Vec2::from(c1.into()),
        center2: ffi::b2Vec2::from(c2.into()),
        radius,
    }
}

/// Axis-aligned box polygon helper
pub fn box_polygon(half_width: f32, half_height: f32) -> ffi::b2Polygon {
    unsafe { ffi::b2MakeBox(half_width, half_height) }
}

/// Build a polygon from an arbitrary set of points by computing the convex hull
/// and applying a radius. Returns None if the input is empty.
pub fn polygon_from_points<I, P>(points: I, radius: f32) -> Option<ffi::b2Polygon>
where
    I: IntoIterator<Item = P>,
    P: Into<crate::types::Vec2>,
{
    let pts: Vec<ffi::b2Vec2> = points
        .into_iter()
        .map(|p| ffi::b2Vec2::from(p.into()))
        .collect();
    if pts.is_empty() {
        return None;
    }
    let hull = unsafe { ffi::b2ComputeHull(pts.as_ptr(), pts.len() as i32) };
    let poly = unsafe { ffi::b2MakePolygon(&hull, radius) };
    Some(poly)
}
