use super::*;

impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            def: MotorJointDef::new(JointBase::new(body_a, body_b)),
        }
    }

    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            def: FilterJointDef::new(JointBase::new(body_a, body_b)),
        }
    }
}
