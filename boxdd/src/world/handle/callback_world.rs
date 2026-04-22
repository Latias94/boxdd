use super::*;

impl CallbackWorld {
    pub(crate) fn new(core: Arc<WorldCore>) -> Self {
        Self { core }
    }

    pub fn with_body_user_data<T: 'static + Sync, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_body_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_body_user_data<T: 'static + Sync, R>(
        &self,
        id: BodyId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_body_user_data(id, f)
    }

    pub fn with_shape_user_data<T: 'static + Sync, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_shape_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_shape_user_data<T: 'static + Sync, R>(
        &self,
        id: ShapeId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_shape_user_data(id, f)
    }

    pub fn with_joint_user_data<T: 'static + Sync, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.core
            .try_with_joint_user_data(id, f)
            .expect("user data type mismatch")
    }

    pub fn try_with_joint_user_data<T: 'static + Sync, R>(
        &self,
        id: JointId,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_joint_user_data(id, f)
    }

    pub fn with_world_user_data<T: 'static + Sync, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.core
            .try_with_world_user_data(f)
            .expect("user data type mismatch")
    }

    pub fn try_with_world_user_data<T: 'static + Sync, R>(
        &self,
        f: impl FnOnce(&T) -> R,
    ) -> crate::error::ApiResult<Option<R>> {
        self.core.try_with_world_user_data(f)
    }
}
