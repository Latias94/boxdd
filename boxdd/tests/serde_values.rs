#![cfg(feature = "serde")]

use boxdd::{Aabb, QueryFilter, Vec2};

#[test]
fn aabb_serde_roundtrip() {
    let a = Aabb::new(Vec2::new(-1.0, -2.0), Vec2::new(3.0, 4.0));
    let s = serde_json::to_string(&a).unwrap();
    let b: Aabb = serde_json::from_str(&s).unwrap();
    assert_eq!(a, b);
}

#[test]
fn query_filter_serde_roundtrip() {
    let q = QueryFilter::default().category(0x11).mask(0x22);
    let s = serde_json::to_string(&q).unwrap();
    let q2: QueryFilter = serde_json::from_str(&s).unwrap();
    assert_eq!(q.category_bits(), q2.category_bits());
    assert_eq!(q.mask_bits(), q2.mask_bits());
}
