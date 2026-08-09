use boxdd::BodyId;

fn require_deserialize<T: for<'de> serde::Deserialize<'de>>() {}

fn main() {
    require_deserialize::<BodyId>();
}
