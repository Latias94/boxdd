use super::*;

impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        RevoluteJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: RevoluteJointDef::new(JointBase::default()),
        }
    }

    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        PrismaticJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(JointBase::default()),
        }
    }

    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        WheelJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(JointBase::default()),
        }
    }

    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        DistanceJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(JointBase::default()),
        }
    }

    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        WeldJointBuilder {
            world: self,
            body_a,
            body_b,
            anchor_world: None,
            def: WeldJointDef::new(JointBase::default()),
        }
    }

    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        MotorJointBuilder {
            world: self,
            body_a,
            body_b,
            def: MotorJointDef::new(JointBase::default()),
        }
    }

    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        FilterJointBuilder {
            world: self,
            body_a,
            body_b,
            def: FilterJointDef::new(JointBase::default()),
        }
    }
}
