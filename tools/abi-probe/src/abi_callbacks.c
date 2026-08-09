#include <box2d/box2d.h>

#include <stddef.h>
#include <stdint.h>

#if defined(_MSC_VER)
#define BOXDD_ALIGNOF(type) __alignof(type)
#else
#define BOXDD_ALIGNOF(type) _Alignof(type)
#endif

size_t boxdd_abi_probe_tree_node_size(void)
{
	return sizeof(b2TreeNode);
}

size_t boxdd_abi_probe_tree_node_alignment(void)
{
	return BOXDD_ALIGNOF(b2TreeNode);
}

size_t boxdd_abi_probe_tree_node_aabb_offset(void)
{
	return offsetof(b2TreeNode, aabb);
}

size_t boxdd_abi_probe_tree_node_category_bits_offset(void)
{
	return offsetof(b2TreeNode, categoryBits);
}

size_t boxdd_abi_probe_tree_node_children_offset(void)
{
	return offsetof(b2TreeNode, children);
}

size_t boxdd_abi_probe_tree_node_user_data_offset(void)
{
	return offsetof(b2TreeNode, userData);
}

size_t boxdd_abi_probe_tree_node_parent_offset(void)
{
	return offsetof(b2TreeNode, parent);
}

size_t boxdd_abi_probe_tree_node_next_offset(void)
{
	return offsetof(b2TreeNode, next);
}

size_t boxdd_abi_probe_tree_node_height_offset(void)
{
	return offsetof(b2TreeNode, height);
}

size_t boxdd_abi_probe_tree_node_flags_offset(void)
{
	return offsetof(b2TreeNode, flags);
}

bool boxdd_abi_probe_invoke_alloc(b2AllocFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	void* memory = callback((size_t)37, 16);
	return memory != NULL && ((uintptr_t)memory % (uintptr_t)16) == (uintptr_t)0;
}

bool boxdd_abi_probe_invoke_free(b2FreeFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	uint64_t memory = UINT64_C(0x123456789ABCDEF0);
	callback(&memory, (size_t)41);
	return memory == UINT64_C(0x123456789ABCDEF0);
}

int boxdd_abi_probe_invoke_assert(b2AssertFcn* callback)
{
	if (callback == NULL)
	{
		return -1;
	}
	return callback("boxdd-condition", "boxdd-file.c", 73);
}

bool boxdd_abi_probe_invoke_log(b2LogFcn* callback)
{
	if (callback == NULL)
	{
		return false;
	}
	callback("boxdd-log-message");
	return true;
}

bool boxdd_abi_probe_invoke_task(b2TaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	callback(context);
	return true;
}

static void boxdd_abi_probe_nested_task(void* taskContext)
{
	uint32_t* calls = (uint32_t*)taskContext;
	*calls += 1u;
}

uint32_t boxdd_abi_probe_invoke_enqueue_task(b2EnqueueTaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return 0u;
	}
	uint32_t nestedCalls = 0u;
	void* result = callback(boxdd_abi_probe_nested_task, &nestedCalls, context);
	uint32_t status = 0u;
	status |= nestedCalls == 1u ? 1u : 0u;
	status |= result == NULL ? 2u : 0u;
	return status;
}

bool boxdd_abi_probe_invoke_finish_task(b2FinishTaskCallback* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	uint64_t userTask = UINT64_C(0xA5A55A5AF0F00F0F);
	callback(&userTask, context);
	return userTask == UINT64_C(0xA5A55A5AF0F00F0F);
}

float boxdd_abi_probe_invoke_friction(b2FrictionCallback* callback)
{
	if (callback == NULL)
	{
		return -1.0f;
	}
	return callback(0.25f, UINT64_C(101), 0.75f, UINT64_C(202));
}

float boxdd_abi_probe_invoke_restitution(b2RestitutionCallback* callback)
{
	if (callback == NULL)
	{
		return -1.0f;
	}
	return callback(0.125f, UINT64_C(303), 0.875f, UINT64_C(404));
}

bool boxdd_abi_probe_invoke_tree_query(b2TreeQueryCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	return callback(7, UINT64_C(11), context);
}

float boxdd_abi_probe_invoke_tree_ray_cast(b2TreeRayCastCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2RayCastInput input = {0};
	input.origin.x = 1.25f;
	input.origin.y = -2.5f;
	input.translation.x = 3.5f;
	input.translation.y = -4.5f;
	input.maxFraction = 0.75f;
	return callback(&input, 13, UINT64_C(17), context);
}

float boxdd_abi_probe_invoke_tree_box_cast(b2TreeBoxCastCallbackFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2BoxCastInput input = {0};
	input.box.lowerBound.x = -1.25f;
	input.box.lowerBound.y = -2.25f;
	input.box.upperBound.x = 3.25f;
	input.box.upperBound.y = 4.25f;
	input.translation.x = 5.5f;
	input.translation.y = -6.5f;
	input.maxFraction = 0.875f;
	return callback(&input, 19, UINT64_C(23), context);
}

bool boxdd_abi_probe_invoke_custom_filter(b2CustomFilterFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shapeA = {0};
	shapeA.index1 = 29;
	shapeA.world0 = 31;
	shapeA.generation = 37;
	b2ShapeId shapeB = {0};
	shapeB.index1 = 41;
	shapeB.world0 = 43;
	shapeB.generation = 47;
	return callback(shapeA, shapeB, context);
}

bool boxdd_abi_probe_invoke_pre_solve(b2PreSolveFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shapeA = {0};
	shapeA.index1 = 53;
	shapeA.world0 = 59;
	shapeA.generation = 61;
	b2ShapeId shapeB = {0};
	shapeB.index1 = 67;
	shapeB.world0 = 71;
	shapeB.generation = 73;
	b2Pos point = {0};
	point.x = 1.25;
	point.y = -2.5;
	b2Vec2 normal = {0};
	normal.x = 0.0f;
	normal.y = 1.0f;
	return callback(shapeA, shapeB, point, normal, context);
}

bool boxdd_abi_probe_invoke_overlap_result(b2OverlapResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shape = {0};
	shape.index1 = 79;
	shape.world0 = 83;
	shape.generation = 89;
	return callback(shape, context);
}

float boxdd_abi_probe_invoke_cast_result(b2CastResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return -1.0f;
	}
	b2ShapeId shape = {0};
	shape.index1 = 97;
	shape.world0 = 101;
	shape.generation = 103;
	b2Pos point = {0};
	point.x = 2.25;
	point.y = -3.5;
	b2Vec2 normal = {0};
	normal.x = -0.5f;
	normal.y = 1.0f;
	return callback(shape, point, normal, 0.375f, context);
}

bool boxdd_abi_probe_invoke_plane_result(b2PlaneResultFcn* callback, void* context)
{
	if (callback == NULL || context == NULL)
	{
		return false;
	}
	b2ShapeId shape = {0};
	shape.index1 = 107;
	shape.world0 = 109;
	shape.generation = 113;
	b2PlaneResult plane = {0};
	plane.plane.normal.x = 0.0f;
	plane.plane.normal.y = 1.0f;
	plane.plane.offset = 1.25f;
	plane.point.x = -2.5f;
	plane.point.y = 3.75f;
	plane.hit = true;
	return callback(shape, &plane, context);
}

b2Version boxdd_abi_probe_get_version(void)
{
	return b2GetVersion();
}

bool boxdd_abi_probe_precision_matches(void)
{
#if defined(BOX2D_DOUBLE_PRECISION)
	return b2IsDoublePrecision();
#else
	return !b2IsDoublePrecision();
#endif
}
