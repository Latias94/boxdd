#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PortStyle {
    TeachingAdaptation,
}

impl PortStyle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TeachingAdaptation => "TeachingAdaptation",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct UpstreamSampleRef {
    pub category: &'static str,
    pub name: &'static str,
    pub style: PortStyle,
}

#[derive(Copy, Clone)]
pub struct TestbedSceneMetadata {
    pub scene: TestbedScene,
    pub id: &'static str,
    pub category: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub upstream: &'static [UpstreamSampleRef],
}

macro_rules! define_scene_registry {
    ($(
        TestbedSceneMetadata {
            scene: TestbedScene::$variant:ident,
            id: $id:expr,
            category: $category:expr,
            name: $name:expr,
            description: $description:expr,
            upstream: $upstream:expr $(,)?
        }
    ),+ $(,)?) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq)]
        pub enum TestbedScene {
            $($variant),+
        }

        pub const SCENE_REGISTRY: &[TestbedSceneMetadata] = &[
            $(TestbedSceneMetadata {
                scene: TestbedScene::$variant,
                id: $id,
                category: $category,
                name: $name,
                description: $description,
                upstream: $upstream,
            }),+
        ];
    };
}

impl TestbedSceneMetadata {
    pub const fn source_label(self) -> &'static str {
        "official Box2D sample"
    }
}

impl TestbedScene {
    pub fn from_id(id: &str) -> Option<Self> {
        SCENE_REGISTRY
            .iter()
            .find(|metadata| metadata.id == id)
            .map(|metadata| metadata.scene)
    }

    pub fn index(self) -> usize {
        SCENE_REGISTRY
            .iter()
            .position(|metadata| metadata.scene == self)
            .expect("testbed scene missing from SCENE_REGISTRY")
    }
}

define_scene_registry![
    TestbedSceneMetadata {
        scene: TestbedScene::SingleBox,
        id: "single-box",
        category: "Stacking",
        name: "Single Box",
        description: "A dynamic box starts with horizontal velocity and settles on a long static segment.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Single Box",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::TiltedStack,
        id: "tilted-stack",
        category: "Stacking",
        name: "Tilted Stack",
        description: "Offset columns of rounded boxes show solver stability under uneven stacking pressure.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Tilted Stack",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::CircleStack,
        id: "circle-stack",
        category: "Stacking",
        name: "Circle Stack",
        description: "Dynamic circles stack and roll through the same contact solver path as the official sample.",
        upstream: &[UpstreamSampleRef {
            category: "Stacking",
            name: "Circle Stack",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Pyramid,
        id: "pyramid",
        category: "Benchmark",
        name: "Large Pyramid",
        description: "A browser-sized version of the classic Box2D pyramid solver stress sample.",
        upstream: &[UpstreamSampleRef {
            category: "Benchmark",
            name: "Large Pyramid",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::BodyType,
        id: "body-type",
        category: "Bodies",
        name: "Body Type",
        description: "Static, kinematic, and dynamic bodies share one scene so body behavior is visible.",
        upstream: &[UpstreamSampleRef {
            category: "Bodies",
            name: "Body Type",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::KinematicPlatform,
        id: "kinematic-platform",
        category: "Bodies",
        name: "Kinematic",
        description: "An app-controlled platform drives dynamic boxes through Bevy-to-Box2D transform sync.",
        upstream: &[UpstreamSampleRef {
            category: "Bodies",
            name: "Kinematic",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ContinuousBullet,
        id: "continuous-bullet",
        category: "Continuous",
        name: "Skinny Box",
        description: "A fast bullet body targets a thin wall with continuous collision enabled.",
        upstream: &[UpstreamSampleRef {
            category: "Continuous",
            name: "Skinny Box",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Restitution,
        id: "restitution",
        category: "Shapes",
        name: "Restitution",
        description: "Identical circles fall onto pads with increasing restitution values.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Restitution",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::Friction,
        id: "friction",
        category: "Shapes",
        name: "Friction",
        description: "Boxes slide across ramps and floors with different friction coefficients.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Friction",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ShapeFilter,
        id: "shape-filter",
        category: "Shapes",
        name: "Filter",
        description: "Category and mask bits split bodies into groups that collide or pass through each other.",
        upstream: &[UpstreamSampleRef {
            category: "Shapes",
            name: "Filter",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::SensorFunnel,
        id: "sensor-funnel",
        category: "Events",
        name: "Sensor Funnel",
        description: "Falling visitors pass through a transparent sensor and update the egui counters.",
        upstream: &[UpstreamSampleRef {
            category: "Events",
            name: "Sensor Funnel",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::ContactEvents,
        id: "contact-event",
        category: "Events",
        name: "Contact",
        description: "Contact begin, end, and hit events are enabled on dynamic bodies and reflected in the panel.",
        upstream: &[UpstreamSampleRef {
            category: "Events",
            name: "Contact",
            style: PortStyle::TeachingAdaptation,
        }],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::DistanceBridge,
        id: "bridge",
        category: "Joints",
        name: "Bridge",
        description: "Distance joints connect planks into a bridge and a dropped weight disturbs the chain.",
        upstream: &[
            UpstreamSampleRef {
                category: "Joints",
                name: "Distance Joint",
                style: PortStyle::TeachingAdaptation,
            },
            UpstreamSampleRef {
                category: "Joints",
                name: "Bridge",
                style: PortStyle::TeachingAdaptation,
            },
        ],
    },
    TestbedSceneMetadata {
        scene: TestbedScene::RevolutePendulum,
        id: "revolute",
        category: "Joints",
        name: "Revolute",
        description: "A revolute joint creates a pendulum that strikes a small stack.",
        upstream: &[UpstreamSampleRef {
            category: "Joints",
            name: "Revolute",
            style: PortStyle::TeachingAdaptation,
        }],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_the_unique_scene_order_authority() {
        for (index, metadata) in SCENE_REGISTRY.iter().enumerate() {
            assert_eq!(metadata.scene.index(), index);
            assert_eq!(TestbedScene::from_id(metadata.id), Some(metadata.scene));
        }
    }
}
