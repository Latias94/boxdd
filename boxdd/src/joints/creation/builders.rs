use super::*;

impl World {
    pub fn revolute<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> RevoluteJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        RevoluteJointBuilder {
            world: self,
            anchor_world: None,
            def: RevoluteJointDef::new(base),
        }
    }

    pub fn prismatic<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> PrismaticJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        PrismaticJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: PrismaticJointDef::new(base),
        }
    }

    pub fn wheel<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WheelJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        WheelJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            axis_world: None,
            def: WheelJointDef::new(base),
        }
    }

    pub fn distance<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> DistanceJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        DistanceJointBuilder {
            world: self,
            anchor_a_world: None,
            anchor_b_world: None,
            def: DistanceJointDef::new(base),
        }
    }

    pub fn weld<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> WeldJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        WeldJointBuilder {
            world: self,
            anchor_world: None,
            def: WeldJointDef::new(base),
        }
    }

    pub fn motor_joint<'w>(&'w mut self, body_a: BodyId, body_b: BodyId) -> MotorJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        MotorJointBuilder {
            world: self,
            def: MotorJointDef::new(base),
        }
    }

    pub fn filter_joint<'w>(
        &'w mut self,
        body_a: BodyId,
        body_b: BodyId,
    ) -> FilterJointBuilder<'w> {
        let base = self.core().joint_base(body_a, body_b);
        FilterJointBuilder {
            world: self,
            def: FilterJointDef::new(base),
        }
    }
}
