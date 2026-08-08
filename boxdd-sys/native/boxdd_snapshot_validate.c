// SPDX-License-Identifier: MIT OR Apache-2.0

#include "boxdd_adapter.h"

#include "body.h"
#include "broad_phase.h"
#include "constraint_graph.h"
#include "contact.h"
#include "island.h"
#include "joint.h"
#include "sensor.h"
#include "shape.h"
#include "solver_set.h"
#include "table.h"

#include "box2d/collision.h"
#include "box2d/types.h"

#include <limits.h>
#include <math.h>
#include <stddef.h>
#include <string.h>

#define BOXDD_SNAPSHOT_MAGIC 0x32534E42u
#define BOXDD_SNAPSHOT_VERSION 3u
#define BOXDD_SNAPSHOT_VALIDATION_FLAG 0x1u
#define BOXDD_SNAPSHOT_DOUBLE_FLAG 0x2u
#define BOXDD_SNAPSHOT_KNOWN_FLAGS 0x3u
#define BOXDD_NULL_INDEX ( -1 )
#define BOXDD_GRAPH_COLOR_COUNT 24
#define BOXDD_OVERFLOW_COLOR 23

typedef struct boxddPoolImage
{
	int32_t nextIndex;
	int32_t freeCount;
	const uint8_t* freeIds;
} boxddPoolImage;

typedef struct boxddSnapshotCursor
{
	const uint8_t* image;
	size_t size;
	size_t cursor;
	uint64_t work;
	boxddSnapshotStatus status;
	const boxddSnapshotLimits* limits;
} boxddSnapshotCursor;

typedef struct boxddSnapshotContext
{
	boxddSnapshotCursor cursor;
	boxddSnapshotFacts facts;
	boxddSnapshotEntry* entries;
	size_t entryCapacity;
	size_t bases[7];
	boxddPoolImage pools[7];
	size_t solverOffset;
	const uint8_t* bodies;
	const uint8_t* shapes;
	const uint8_t* contacts;
	const uint8_t* joints;
	int32_t bodyCount;
	int32_t shapeCount;
	int32_t contactCount;
	int32_t jointCount;
	int32_t sensorCount;
	const uint8_t* treeNodes[3];
	int32_t treeCapacities[3];
} boxddSnapshotContext;

static const boxddSnapshotLimits boxddDefaultLimits = {
	(uint32_t)sizeof( boxddSnapshotLimits ), BOXDD_SNAPSHOT_FACTS_VERSION, 256ull * 1024ull * 1024ull, 16000000ull,
	1000000u, 1000000u, 1000000u, 1000000u, 1000000u, 0u,
};

static void boxddFail( boxddSnapshotCursor* cursor, boxddSnapshotStatus status )
{
	if ( cursor->status == BOXDD_SNAPSHOT_OK )
	{
		cursor->status = status;
	}
}

static bool boxddCharge( boxddSnapshotCursor* cursor, uint64_t amount )
{
	if ( amount > UINT64_MAX - cursor->work )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_OVERFLOW );
		return false;
	}
	cursor->work += amount;
	if ( cursor->work > cursor->limits->maxValidationWork )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_LIMIT_EXCEEDED );
		return false;
	}
	return true;
}

static const uint8_t* boxddTake( boxddSnapshotCursor* cursor, size_t bytes )
{
	if ( cursor->status != BOXDD_SNAPSHOT_OK )
	{
		return NULL;
	}
	if ( bytes > cursor->size - cursor->cursor )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_TRUNCATED );
		return NULL;
	}
	const uint8_t* result = cursor->image + cursor->cursor;
	cursor->cursor += bytes;
	return result;
}

static bool boxddArrayBytes( boxddSnapshotCursor* cursor, uint64_t count, size_t elementSize, size_t* bytes )
{
	if ( count > cursor->limits->maxArrayElements )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_LIMIT_EXCEEDED );
		return false;
	}
	if ( elementSize != 0 && count > SIZE_MAX / elementSize )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_OVERFLOW );
		return false;
	}
	*bytes = (size_t)count * elementSize;
	return boxddCharge( cursor, count );
}

static uint16_t boxddRawU16( const uint8_t* bytes, size_t offset )
{
	uint16_t value;
	memcpy( &value, bytes + offset, sizeof( value ) );
	return value;
}

static uint32_t boxddRawU32( const uint8_t* bytes, size_t offset )
{
	uint32_t value;
	memcpy( &value, bytes + offset, sizeof( value ) );
	return value;
}

static uint64_t boxddRawU64( const uint8_t* bytes, size_t offset )
{
	uint64_t value;
	memcpy( &value, bytes + offset, sizeof( value ) );
	return value;
}

static int32_t boxddRawI32( const uint8_t* bytes, size_t offset )
{
	int32_t value;
	memcpy( &value, bytes + offset, sizeof( value ) );
	return value;
}

static float boxddRawF32( const uint8_t* bytes, size_t offset )
{
	float value;
	memcpy( &value, bytes + offset, sizeof( value ) );
	return value;
}

static uint8_t boxddRawU8( const uint8_t* bytes, size_t offset )
{
	return bytes[offset];
}

static int32_t boxddReadI32( boxddSnapshotCursor* cursor )
{
	const uint8_t* bytes = boxddTake( cursor, sizeof( int32_t ) );
	return bytes != NULL ? boxddRawI32( bytes, 0 ) : 0;
}

static uint32_t boxddReadU32( boxddSnapshotCursor* cursor )
{
	const uint8_t* bytes = boxddTake( cursor, sizeof( uint32_t ) );
	return bytes != NULL ? boxddRawU32( bytes, 0 ) : 0;
}

static const uint8_t* boxddReadArray( boxddSnapshotCursor* cursor, int32_t* count, size_t elementSize )
{
	*count = boxddReadI32( cursor );
	if ( cursor->status != BOXDD_SNAPSHOT_OK )
	{
		return NULL;
	}
	if ( *count < 0 )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return NULL;
	}
	size_t bytes = 0;
	if ( boxddArrayBytes( cursor, (uint32_t)*count, elementSize, &bytes ) == false )
	{
		return NULL;
	}
	return boxddTake( cursor, bytes );
}

static bool boxddBytesAreZero( const uint8_t* bytes, size_t count )
{
	for ( size_t i = 0; i < count; ++i )
	{
		if ( bytes[i] != 0 )
		{
			return false;
		}
	}
	return true;
}

static bool boxddCanonicalBool( boxddSnapshotCursor* cursor, const uint8_t* bytes, size_t offset )
{
	uint8_t value = boxddRawU8( bytes, offset );
	if ( value > 1u )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return false;
	}
	return true;
}

static bool boxddValidateContactSimBools( boxddSnapshotCursor* cursor, const uint8_t* sim )
{
	size_t pointsOffset = offsetof( b2ContactSim, manifold ) + offsetof( b2Manifold, points );
	for ( size_t point = 0; point < 2; ++point )
	{
		size_t persistedOffset = pointsOffset + point * sizeof( b2ManifoldPoint ) + offsetof( b2ManifoldPoint, persisted );
		if ( !boxddCanonicalBool( cursor, sim, persistedOffset ) )
		{
			return false;
		}
	}
	return true;
}

static bool boxddValidateJointSimBools( boxddSnapshotCursor* cursor, const uint8_t* sim )
{
	uint32_t type = boxddRawU32( sim, offsetof( b2JointSim, type ) );
	size_t baseOffset;
	size_t springOffset;
	size_t limitOffset;
	size_t motorOffset;
	switch ( type )
	{
		case b2_distanceJoint:
			baseOffset = offsetof( b2JointSim, distanceJoint );
			springOffset = offsetof( b2DistanceJoint, enableSpring );
			limitOffset = offsetof( b2DistanceJoint, enableLimit );
			motorOffset = offsetof( b2DistanceJoint, enableMotor );
			break;

		case b2_prismaticJoint:
			baseOffset = offsetof( b2JointSim, prismaticJoint );
			springOffset = offsetof( b2PrismaticJoint, enableSpring );
			limitOffset = offsetof( b2PrismaticJoint, enableLimit );
			motorOffset = offsetof( b2PrismaticJoint, enableMotor );
			break;

		case b2_revoluteJoint:
			baseOffset = offsetof( b2JointSim, revoluteJoint );
			springOffset = offsetof( b2RevoluteJoint, enableSpring );
			limitOffset = offsetof( b2RevoluteJoint, enableLimit );
			motorOffset = offsetof( b2RevoluteJoint, enableMotor );
			break;

		case b2_wheelJoint:
			baseOffset = offsetof( b2JointSim, wheelJoint );
			springOffset = offsetof( b2WheelJoint, enableSpring );
			limitOffset = offsetof( b2WheelJoint, enableLimit );
			motorOffset = offsetof( b2WheelJoint, enableMotor );
			break;

		case b2_filterJoint:
		case b2_motorJoint:
		case b2_weldJoint:
			return true;

		default:
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return false;
	}

	return boxddCanonicalBool( cursor, sim, baseOffset + springOffset ) &&
		   boxddCanonicalBool( cursor, sim, baseOffset + limitOffset ) &&
		   boxddCanonicalBool( cursor, sim, baseOffset + motorOffset );
}

static bool boxddValidIndex( int32_t index, int32_t count )
{
	return index == BOXDD_NULL_INDEX || ( index >= 0 && index < count );
}

static boxddSnapshotEntry* boxddEntry( boxddSnapshotContext* context, uint32_t kind, int32_t index )
{
	if ( context->entries == NULL || kind < BOXDD_SNAPSHOT_ENTRY_BODY || kind > BOXDD_SNAPSHOT_ENTRY_SOLVER_SET || index < 0 )
	{
		return NULL;
	}
	uint32_t slot = kind - 1u;
	if ( (uint32_t)index >= context->facts.poolNext[slot] )
	{
		return NULL;
	}
	return context->entries + context->bases[slot] + (size_t)index;
}

static bool boxddEntryIsLive( boxddSnapshotContext* context, uint32_t kind, int32_t index )
{
	boxddSnapshotEntry* entry = boxddEntry( context, kind, index );
	return entry != NULL && ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) != 0;
}

static bool boxddValidateReferenceAt( boxddSnapshotContext* context, boxddSnapshotCursor* cursor, uint32_t kind, int32_t index,
									 bool nullable )
{
	if ( nullable && index == BOXDD_NULL_INDEX )
	{
		return true;
	}
	if ( kind < BOXDD_SNAPSHOT_ENTRY_BODY || kind > BOXDD_SNAPSHOT_ENTRY_SOLVER_SET || index < 0 ||
		 (uint32_t)index >= context->facts.poolNext[kind - 1u] )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	if ( context->entries == NULL )
	{
		return true;
	}
	if ( !boxddEntryIsLive( context, kind, index ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	return true;
}

static bool boxddValidateReference( boxddSnapshotContext* context, uint32_t kind, int32_t index, bool nullable )
{
	return boxddValidateReferenceAt( context, &context->cursor, kind, index, nullable );
}

static boxddSnapshotEntry* boxddValidateSimLocation( boxddSnapshotContext* context, boxddSnapshotCursor* cursor, uint32_t kind,
												 int32_t id, int32_t setIndex, int32_t colorIndex, int32_t localIndex,
												 bool* valid )
{
	*valid = false;
	if ( !boxddValidateReferenceAt( context, cursor, kind, id, false ) )
	{
		return NULL;
	}
	if ( context->entries == NULL )
	{
		*valid = true;
		return NULL;
	}

	boxddSnapshotEntry* entry = boxddEntry( context, kind, id );
	if ( entry == NULL || entry->setIndex != setIndex || entry->colorIndex != colorIndex || entry->localIndex != localIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return NULL;
	}

	*valid = true;
	return entry;
}

static bool boxddValidateContactSimRelation( boxddSnapshotContext* context, boxddSnapshotCursor* cursor, const uint8_t* sim,
												 int32_t setIndex, int32_t colorIndex, int32_t localIndex )
{
	int32_t contactId = boxddRawI32( sim, offsetof( b2ContactSim, contactId ) );
	int32_t shapeIdA = boxddRawI32( sim, offsetof( b2ContactSim, shapeIdA ) );
	int32_t shapeIdB = boxddRawI32( sim, offsetof( b2ContactSim, shapeIdB ) );
	if ( !boxddValidateReferenceAt( context, cursor, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeIdA, false ) ||
		 !boxddValidateReferenceAt( context, cursor, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeIdB, false ) )
	{
		return false;
	}

	bool valid = false;
	boxddSnapshotEntry* contact = boxddValidateSimLocation( context, cursor, BOXDD_SNAPSHOT_ENTRY_CONTACT, contactId, setIndex,
															 colorIndex, localIndex, &valid );
	if ( !valid )
	{
		return false;
	}
	if ( contact != NULL && ( contact->ownerA != shapeIdA || contact->ownerB != shapeIdB ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	return true;
}

static bool boxddValidateJointSimRelation( boxddSnapshotContext* context, boxddSnapshotCursor* cursor, const uint8_t* sim,
											  int32_t setIndex, int32_t colorIndex, int32_t localIndex )
{
	int32_t jointId = boxddRawI32( sim, offsetof( b2JointSim, jointId ) );
	int32_t bodyIdA = boxddRawI32( sim, offsetof( b2JointSim, bodyIdA ) );
	int32_t bodyIdB = boxddRawI32( sim, offsetof( b2JointSim, bodyIdB ) );
	uint32_t type = boxddRawU32( sim, offsetof( b2JointSim, type ) );
	if ( !boxddValidateReferenceAt( context, cursor, BOXDD_SNAPSHOT_ENTRY_BODY, bodyIdA, false ) ||
		 !boxddValidateReferenceAt( context, cursor, BOXDD_SNAPSHOT_ENTRY_BODY, bodyIdB, false ) )
	{
		return false;
	}

	bool valid = false;
	boxddSnapshotEntry* joint = boxddValidateSimLocation( context, cursor, BOXDD_SNAPSHOT_ENTRY_JOINT, jointId, setIndex, colorIndex,
														 localIndex, &valid );
	if ( !valid )
	{
		return false;
	}
	if ( joint != NULL && ( joint->ownerA != bodyIdA || joint->ownerB != bodyIdB || joint->subtype != type ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	return true;
}

static bool boxddValidatePoolSlot( boxddSnapshotContext* context, uint32_t kind, int32_t index, int32_t serializedId )
{
	boxddSnapshotEntry* entry = boxddEntry( context, kind, index );
	if ( entry == NULL )
	{
		return context->entries == NULL;
	}
	bool live = ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) != 0;
	if ( ( live && serializedId != index ) || ( !live && serializedId != BOXDD_NULL_INDEX ) )
	{
		boxddFail( &context->cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	return true;
}

static boxddSnapshotStatus boxddValidateLimits( const boxddSnapshotLimits* limits )
{
	if ( limits->structSize != sizeof( *limits ) || limits->version != BOXDD_SNAPSHOT_FACTS_VERSION || limits->maxImageBytes < 16u ||
		 limits->maxValidationWork == 0 || limits->maxEntries == 0 || limits->maxArrayElements == 0 || limits->maxTreeNodes == 0 ||
		 limits->maxHashCapacity == 0 || limits->maxBitsetBlocks == 0 )
	{
		return BOXDD_SNAPSHOT_INVALID_VALUE;
	}
	return BOXDD_SNAPSHOT_OK;
}

static void boxddParseWorldConfig( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	const uint8_t* gravity = boxddTake( cursor, sizeof( b2Vec2 ) );
	if ( gravity == NULL )
	{
		return;
	}
	if ( !isfinite( boxddRawF32( gravity, 0 ) ) || !isfinite( boxddRawF32( gravity, sizeof( float ) ) ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return;
	}
	for ( int i = 0; i < 7; ++i )
	{
		const uint8_t* scalar = boxddTake( cursor, sizeof( float ) );
		if ( scalar == NULL || !isfinite( boxddRawF32( scalar, 0 ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
	}
	boxddTake( cursor, sizeof( uint64_t ) );
	boxddReadI32( cursor );
	for ( int i = 0; i < 2; ++i )
	{
		const uint8_t* scalar = boxddTake( cursor, sizeof( float ) );
		if ( scalar == NULL || !isfinite( boxddRawF32( scalar, 0 ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
	}
	int32_t endEventIndex = boxddReadI32( cursor );
	if ( endEventIndex != 0 && endEventIndex != 1 )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return;
	}
	const uint8_t* capacities = boxddTake( cursor, sizeof( b2Capacity ) );
	if ( capacities == NULL )
	{
		return;
	}
	for ( size_t offset = 0; offset < sizeof( b2Capacity ); offset += sizeof( int32_t ) )
	{
		if ( boxddRawI32( capacities, offset ) < 0 )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
	}
	const uint8_t* flags = boxddTake( cursor, 1 );
	if ( flags == NULL || ( flags[0] & ~0x1Fu ) != 0 )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return;
	}
	context->facts.worldFlags = flags[0];
}

static void boxddParsePools( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	size_t required = 0;
	for ( uint32_t poolIndex = 0; poolIndex < 7 && cursor->status == BOXDD_SNAPSHOT_OK; ++poolIndex )
	{
		int32_t nextIndex = boxddReadI32( cursor );
		int32_t freeCount = 0;
		const uint8_t* freeIds = boxddReadArray( cursor, &freeCount, sizeof( int32_t ) );
		if ( nextIndex < 0 || (uint32_t)nextIndex > cursor->limits->maxEntries || freeCount > nextIndex )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( required > SIZE_MAX - (size_t)nextIndex )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_OVERFLOW );
			return;
		}
		context->bases[poolIndex] = required;
		required += (size_t)nextIndex;
		if ( required > cursor->limits->maxEntries )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_LIMIT_EXCEEDED );
			return;
		}
		context->pools[poolIndex] = (boxddPoolImage){ nextIndex, freeCount, freeIds };
		context->facts.poolNext[poolIndex] = (uint32_t)nextIndex;
		context->facts.poolFree[poolIndex] = (uint32_t)freeCount;
		context->facts.entryCounts[poolIndex] = (uint32_t)nextIndex;
	}
	context->facts.requiredEntries = required;
	if ( context->entries == NULL || context->entryCapacity < required )
	{
		// Treat an undersized output exactly like the sizing pass. This prevents semantic
		// validation from observing caller memory that was intentionally left uninitialized.
		context->entries = NULL;
		return;
	}
	for ( uint32_t poolIndex = 0; poolIndex < 7; ++poolIndex )
	{
		boxddPoolImage* pool = context->pools + poolIndex;
		for ( int32_t index = 0; index < pool->nextIndex; ++index )
		{
			boxddSnapshotEntry* entry = context->entries + context->bases[poolIndex] + (size_t)index;
			*entry = (boxddSnapshotEntry){ 0 };
			entry->structSize = (uint32_t)sizeof( *entry );
			entry->version = BOXDD_SNAPSHOT_ENTRY_VERSION;
			entry->kind = poolIndex + 1u;
			entry->flags = BOXDD_SNAPSHOT_ENTRY_LIVE;
			entry->index = index;
			entry->ownerA = BOXDD_NULL_INDEX;
			entry->ownerB = BOXDD_NULL_INDEX;
			entry->setIndex = BOXDD_NULL_INDEX;
			entry->localIndex = BOXDD_NULL_INDEX;
			entry->colorIndex = BOXDD_NULL_INDEX;
			entry->freeOrder = BOXDD_NULL_INDEX;
			entry->ownerAPrev = BOXDD_NULL_INDEX;
			entry->ownerANext = BOXDD_NULL_INDEX;
			entry->ownerBPrev = BOXDD_NULL_INDEX;
			entry->ownerBNext = BOXDD_NULL_INDEX;
			entry->ownerBOrder = BOXDD_NULL_INDEX;
		}
		for ( int32_t order = 0; order < pool->freeCount; ++order )
		{
			int32_t freeId = boxddRawI32( pool->freeIds, (size_t)order * sizeof( int32_t ) );
			if ( freeId < 0 || freeId >= pool->nextIndex )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
			boxddSnapshotEntry* entry = context->entries + context->bases[poolIndex] + (size_t)freeId;
			if ( ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_DUPLICATE );
				return;
			}
			entry->flags = 0;
			entry->freeOrder = order;
		}
	}
}

static const uint8_t* boxddParsePodArray( boxddSnapshotCursor* cursor, int32_t* count, size_t elementSize )
{
	return boxddReadArray( cursor, count, elementSize );
}

static void boxddParseSolverSets( boxddSnapshotContext* context, boxddSnapshotCursor* cursor, bool validateReferences )
{
	int32_t setCount = boxddReadI32( cursor );
	if ( setCount != context->pools[6].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t setSlot = 0; setSlot < setCount && cursor->status == BOXDD_SNAPSHOT_OK; ++setSlot )
	{
		int32_t setIndex = boxddReadI32( cursor );
		int32_t bodySimCount = 0;
		const uint8_t* bodySims = boxddParsePodArray( cursor, &bodySimCount, sizeof( b2BodySim ) );
		int32_t bodyStateCount = 0;
		boxddParsePodArray( cursor, &bodyStateCount, sizeof( b2BodyState ) );
		int32_t jointSimCount = 0;
		const uint8_t* jointSims = boxddParsePodArray( cursor, &jointSimCount, sizeof( b2JointSim ) );
		int32_t contactSimCount = 0;
		const uint8_t* contactSims = boxddParsePodArray( cursor, &contactSimCount, sizeof( b2ContactSim ) );
		int32_t islandSimCount = 0;
		const uint8_t* islandSims = boxddParsePodArray( cursor, &islandSimCount, sizeof( b2IslandSim ) );
		if ( cursor->status != BOXDD_SNAPSHOT_OK )
		{
			return;
		}
		boxddSnapshotEntry* setEntry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_SOLVER_SET, setSlot );
		if ( setEntry != NULL )
		{
			bool live = ( setEntry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) != 0;
			if ( ( live && setIndex != setSlot ) || ( !live && setIndex != BOXDD_NULL_INDEX ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
			setEntry->setIndex = setIndex;
		}
		if ( setIndex == BOXDD_NULL_INDEX && ( bodySimCount != 0 || bodyStateCount != 0 || jointSimCount != 0 || contactSimCount != 0 || islandSimCount != 0 ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( setIndex == b2_staticSet && ( bodyStateCount != 0 || contactSimCount != 0 || islandSimCount != 0 ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( setIndex == b2_disabledSet && ( bodyStateCount != 0 || islandSimCount != 0 ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( setIndex == b2_awakeSet && ( bodyStateCount != bodySimCount || jointSimCount != 0 ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( setIndex >= b2_firstSleepingSet && bodyStateCount != 0 )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		for ( int32_t i = 0; i < jointSimCount; ++i )
		{
			const uint8_t* sim = jointSims + (size_t)i * sizeof( b2JointSim );
			if ( !boxddValidateJointSimBools( cursor, sim ) )
			{
				return;
			}
		}
		for ( int32_t i = 0; i < contactSimCount; ++i )
		{
			const uint8_t* sim = contactSims + (size_t)i * sizeof( b2ContactSim );
			if ( !boxddValidateContactSimBools( cursor, sim ) )
			{
				return;
			}
		}
		if ( !validateReferences )
		{
			continue;
		}
		for ( int32_t i = 0; i < bodySimCount; ++i )
		{
			const uint8_t* sim = bodySims + (size_t)i * sizeof( b2BodySim );
			int32_t bodyId = boxddRawI32( sim, offsetof( b2BodySim, bodyId ) );
			boxddSnapshotEntry* body = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyId );
			if ( body == NULL || ( body->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 || body->setIndex != setIndex || body->localIndex != i )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
		}
		for ( int32_t i = 0; i < jointSimCount; ++i )
		{
			const uint8_t* sim = jointSims + (size_t)i * sizeof( b2JointSim );
			if ( !boxddValidateJointSimRelation( context, cursor, sim, setIndex, BOXDD_NULL_INDEX, i ) )
			{
				return;
			}
		}
		for ( int32_t i = 0; i < contactSimCount; ++i )
		{
			const uint8_t* sim = contactSims + (size_t)i * sizeof( b2ContactSim );
			if ( !boxddValidateContactSimRelation( context, cursor, sim, setIndex, BOXDD_NULL_INDEX, i ) )
			{
				return;
			}
			int32_t pointCount = boxddRawI32( sim, offsetof( b2ContactSim, manifold ) + offsetof( b2Manifold, pointCount ) );
			if ( pointCount < 0 || pointCount > 2 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
				return;
			}
		}
		for ( int32_t i = 0; i < islandSimCount; ++i )
		{
			const uint8_t* sim = islandSims + (size_t)i * sizeof( b2IslandSim );
			int32_t islandId = boxddRawI32( sim, offsetof( b2IslandSim, islandId ) );
			boxddSnapshotEntry* island = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_ISLAND, islandId );
			if ( island == NULL || ( island->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 || island->setIndex != setIndex || island->localIndex != i )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
		}
	}
}

static void boxddParseBodies( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	context->bodies = boxddReadArray( cursor, &context->bodyCount, sizeof( b2Body ) );
	if ( context->bodyCount != context->pools[0].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < context->bodyCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		const uint8_t* raw = context->bodies + (size_t)i * sizeof( b2Body );
		int32_t id = boxddRawI32( raw, offsetof( b2Body, id ) );
		if ( !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_BODY, i, id ) )
		{
			return;
		}
		if ( !boxddBytesAreZero( raw + offsetof( b2Body, userData ), sizeof( void* ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_BODY, i );
		if ( entry == NULL )
		{
			continue;
		}
		entry->generation = boxddRawU16( raw, offsetof( b2Body, generation ) );
		if ( ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		int32_t type = boxddRawI32( raw, offsetof( b2Body, type ) );
		entry->subtype = (uint32_t)type;
		entry->setIndex = boxddRawI32( raw, offsetof( b2Body, setIndex ) );
		entry->localIndex = boxddRawI32( raw, offsetof( b2Body, localIndex ) );
		entry->ownerA = boxddRawI32( raw, offsetof( b2Body, islandId ) );
		if ( type < 0 || type >= b2_bodyTypeCount || !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SOLVER_SET, entry->setIndex, false ) ||
			 entry->localIndex < 0 || !boxddValidIndex( entry->ownerA, context->pools[5].nextIndex ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
}

static bool boxddValidAabbBytes( const uint8_t* raw )
{
	float lx = boxddRawF32( raw, 0 );
	float ly = boxddRawF32( raw, 4 );
	float ux = boxddRawF32( raw, 8 );
	float uy = boxddRawF32( raw, 12 );
	return isfinite( lx ) && isfinite( ly ) && isfinite( ux ) && isfinite( uy ) && lx <= ux && ly <= uy;
}

static bool boxddAabbContainsBytes( const uint8_t* outer, const uint8_t* inner )
{
	return boxddRawF32( outer, 0 ) <= boxddRawF32( inner, 0 ) && boxddRawF32( outer, 4 ) <= boxddRawF32( inner, 4 ) &&
		   boxddRawF32( inner, 8 ) <= boxddRawF32( outer, 8 ) && boxddRawF32( inner, 12 ) <= boxddRawF32( outer, 12 );
}

static void boxddParseShapes( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	context->shapes = boxddReadArray( cursor, &context->shapeCount, sizeof( b2Shape ) );
	if ( context->shapeCount != context->pools[1].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < context->shapeCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		const uint8_t* raw = context->shapes + (size_t)i * sizeof( b2Shape );
		int32_t id = boxddRawI32( raw, offsetof( b2Shape, id ) );
		if ( !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, i, id ) )
		{
			return;
		}
		if ( !boxddBytesAreZero( raw + offsetof( b2Shape, userData ), sizeof( void* ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		for ( size_t offset = offsetof( b2Shape, enableSensorEvents ); offset <= offsetof( b2Shape, enlargedAABB ); ++offset )
		{
			if ( !boxddCanonicalBool( cursor, raw, offset ) )
			{
				return;
			}
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, i );
		if ( entry == NULL )
		{
			continue;
		}
		entry->generation = boxddRawU16( raw, offsetof( b2Shape, generation ) );
		if ( ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		entry->ownerA = boxddRawI32( raw, offsetof( b2Shape, bodyId ) );
		entry->ownerAPrev = boxddRawI32( raw, offsetof( b2Shape, prevShapeId ) );
		entry->ownerANext = boxddRawI32( raw, offsetof( b2Shape, nextShapeId ) );
		entry->subtype = boxddRawU32( raw, offsetof( b2Shape, type ) );
		if ( entry->subtype >= (uint32_t)b2_shapeTypeCount || !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, entry->ownerA, false ) ||
			 !boxddValidIndex( boxddRawI32( raw, offsetof( b2Shape, prevShapeId ) ), context->shapeCount ) ||
			 !boxddValidIndex( boxddRawI32( raw, offsetof( b2Shape, nextShapeId ) ), context->shapeCount ) ||
			 !boxddValidAabbBytes( raw + offsetof( b2Shape, aabb ) ) || !boxddValidAabbBytes( raw + offsetof( b2Shape, fatAABB ) ) ||
			 !isfinite( boxddRawF32( raw, offsetof( b2Shape, density ) ) ) || boxddRawF32( raw, offsetof( b2Shape, density ) ) < 0.0f )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		if ( boxddRawU8( raw, offsetof( b2Shape, enableCustomFiltering ) ) != 0 )
		{
			entry->flags |= BOXDD_SNAPSHOT_ENTRY_REQUIRES_CUSTOM_FILTER;
			context->facts.requiresCustomFilter = 1;
		}
		if ( boxddRawU8( raw, offsetof( b2Shape, enablePreSolveEvents ) ) != 0 )
		{
			entry->flags |= BOXDD_SNAPSHOT_ENTRY_REQUIRES_PRE_SOLVE;
			context->facts.requiresPreSolve = 1;
		}
	}
}

static void boxddParseContacts( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	context->contacts = boxddReadArray( cursor, &context->contactCount, sizeof( b2Contact ) );
	if ( context->contactCount != context->pools[3].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < context->contactCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		const uint8_t* raw = context->contacts + (size_t)i * sizeof( b2Contact );
		int32_t id = boxddRawI32( raw, offsetof( b2Contact, contactId ) );
		if ( !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_CONTACT, i, id ) )
		{
			return;
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_CONTACT, i );
		if ( entry == NULL )
		{
			continue;
		}
		entry->generation = boxddRawU32( raw, offsetof( b2Contact, generation ) );
		if ( ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		entry->ownerA = boxddRawI32( raw, offsetof( b2Contact, shapeIdA ) );
		entry->ownerB = boxddRawI32( raw, offsetof( b2Contact, shapeIdB ) );
		entry->setIndex = boxddRawI32( raw, offsetof( b2Contact, setIndex ) );
		entry->localIndex = boxddRawI32( raw, offsetof( b2Contact, localIndex ) );
		entry->colorIndex = boxddRawI32( raw, offsetof( b2Contact, colorIndex ) );
		int32_t bodyA = boxddRawI32( raw, offsetof( b2Contact, edges ) + offsetof( b2ContactEdge, bodyId ) );
		int32_t bodyB = boxddRawI32( raw, offsetof( b2Contact, edges ) + sizeof( b2ContactEdge ) + offsetof( b2ContactEdge, bodyId ) );
		if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, entry->ownerA, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, entry->ownerB, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyA, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyB, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SOLVER_SET, entry->setIndex, false ) || entry->localIndex < 0 ||
			 !boxddValidIndex( entry->colorIndex, BOXDD_GRAPH_COLOR_COUNT ) )
		{
			return;
		}
	}
}

static void boxddParseJoints( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	context->joints = boxddReadArray( cursor, &context->jointCount, sizeof( b2Joint ) );
	if ( context->jointCount != context->pools[4].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < context->jointCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		const uint8_t* raw = context->joints + (size_t)i * sizeof( b2Joint );
		int32_t id = boxddRawI32( raw, offsetof( b2Joint, jointId ) );
		if ( !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_JOINT, i, id ) ||
			 !boxddBytesAreZero( raw + offsetof( b2Joint, userData ), sizeof( void* ) ) ||
			 !boxddCanonicalBool( cursor, raw, offsetof( b2Joint, collideConnected ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return;
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_JOINT, i );
		if ( entry == NULL )
		{
			continue;
		}
		entry->generation = boxddRawU16( raw, offsetof( b2Joint, generation ) );
		if ( ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		entry->setIndex = boxddRawI32( raw, offsetof( b2Joint, setIndex ) );
		entry->localIndex = boxddRawI32( raw, offsetof( b2Joint, localIndex ) );
		entry->colorIndex = boxddRawI32( raw, offsetof( b2Joint, colorIndex ) );
		entry->ownerA = boxddRawI32( raw, offsetof( b2Joint, edges ) + offsetof( b2JointEdge, bodyId ) );
		entry->ownerB = boxddRawI32( raw, offsetof( b2Joint, edges ) + sizeof( b2JointEdge ) + offsetof( b2JointEdge, bodyId ) );
		entry->ownerAPrev = boxddRawI32( raw, offsetof( b2Joint, edges ) + offsetof( b2JointEdge, prevKey ) );
		entry->ownerANext = boxddRawI32( raw, offsetof( b2Joint, edges ) + offsetof( b2JointEdge, nextKey ) );
		entry->ownerBPrev =
			boxddRawI32( raw, offsetof( b2Joint, edges ) + sizeof( b2JointEdge ) + offsetof( b2JointEdge, prevKey ) );
		entry->ownerBNext =
			boxddRawI32( raw, offsetof( b2Joint, edges ) + sizeof( b2JointEdge ) + offsetof( b2JointEdge, nextKey ) );
		entry->subtype = boxddRawU32( raw, offsetof( b2Joint, type ) );
		if ( entry->subtype > (uint32_t)b2_wheelJoint || !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, entry->ownerA, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, entry->ownerB, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SOLVER_SET, entry->setIndex, false ) || entry->localIndex < 0 ||
			 !boxddValidIndex( entry->colorIndex, BOXDD_GRAPH_COLOR_COUNT ) )
		{
			return;
		}
	}
}

static void boxddParseChains( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	int32_t chainCount = boxddReadI32( cursor );
	if ( chainCount != context->pools[2].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < chainCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		int32_t id = boxddReadI32( cursor );
		int32_t bodyId = boxddReadI32( cursor );
		int32_t nextChainId = boxddReadI32( cursor );
		int32_t shapeCount = boxddReadI32( cursor );
		int32_t materialCount = boxddReadI32( cursor );
		const uint8_t* generation = boxddTake( cursor, sizeof( uint16_t ) );
		if ( generation == NULL || !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_CHAIN, i, id ) )
		{
			return;
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_CHAIN, i );
		if ( entry != NULL )
		{
			entry->generation = boxddRawU16( generation, 0 );
		}
		if ( id == BOXDD_NULL_INDEX )
		{
			if ( shapeCount != 0 || materialCount != 0 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			}
			continue;
		}
		if ( shapeCount < 1 || materialCount < 1 || materialCount > shapeCount ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyId, false ) || !boxddValidIndex( nextChainId, chainCount ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
		if ( entry != NULL )
		{
			entry->ownerA = bodyId;
			entry->localIndex = nextChainId;
			entry->ownerANext = nextChainId;
			entry->subtype = (uint32_t)materialCount;
			entry->colorIndex = shapeCount;
		}
		size_t shapeBytes = 0;
		size_t materialBytes = 0;
		if ( !boxddArrayBytes( cursor, (uint32_t)shapeCount, sizeof( int32_t ), &shapeBytes ) ||
			 !boxddArrayBytes( cursor, (uint32_t)materialCount, sizeof( b2SurfaceMaterial ), &materialBytes ) )
		{
			return;
		}
		const uint8_t* shapeIds = boxddTake( cursor, shapeBytes );
		const uint8_t* materials = boxddTake( cursor, materialBytes );
		if ( shapeIds == NULL || materials == NULL )
		{
			return;
		}
		for ( int32_t j = 0; j < shapeCount; ++j )
		{
			int32_t shapeId = boxddRawI32( shapeIds, (size_t)j * sizeof( int32_t ) );
			boxddSnapshotEntry* shape = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId );
			if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId, false ) ||
				 ( shape != NULL && ( shape->ownerA != bodyId || shape->subtype != (uint32_t)b2_chainSegmentShape ) ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
			if ( shape != NULL )
			{
				if ( shape->ownerB != BOXDD_NULL_INDEX )
				{
					boxddFail( cursor, BOXDD_SNAPSHOT_DUPLICATE );
					return;
				}
				shape->ownerB = i;
				shape->ownerBOrder = j;
			}
		}
		for ( int32_t j = 0; j < materialCount; ++j )
		{
			const uint8_t* material = materials + (size_t)j * sizeof( b2SurfaceMaterial );
			float friction = boxddRawF32( material, offsetof( b2SurfaceMaterial, friction ) );
			float restitution = boxddRawF32( material, offsetof( b2SurfaceMaterial, restitution ) );
			float rolling = boxddRawF32( material, offsetof( b2SurfaceMaterial, rollingResistance ) );
			float tangent = boxddRawF32( material, offsetof( b2SurfaceMaterial, tangentSpeed ) );
			if ( !isfinite( friction ) || !isfinite( restitution ) || !isfinite( rolling ) || !isfinite( tangent ) || friction < 0.0f ||
				 restitution < 0.0f || rolling < 0.0f )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
				return;
			}
		}
	}
}

static void boxddValidateVisitorArray( boxddSnapshotContext* context )
{
	int32_t count = 0;
	const uint8_t* visitors = boxddReadArray( &context->cursor, &count, sizeof( b2Visitor ) );
	for ( int32_t i = 0; i < count && context->cursor.status == BOXDD_SNAPSHOT_OK; ++i )
	{
		const uint8_t* visitor = visitors + (size_t)i * sizeof( b2Visitor );
		int32_t shapeId = boxddRawI32( visitor, offsetof( b2Visitor, shapeId ) );
		uint16_t generation = boxddRawU16( visitor, offsetof( b2Visitor, generation ) );
		boxddSnapshotEntry* shape = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId );
		if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId, false ) ||
			 ( shape != NULL && shape->generation != generation ) )
		{
			boxddFail( &context->cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
}

static void boxddParseSensors( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	context->sensorCount = boxddReadI32( cursor );
	if ( context->sensorCount < 0 || (uint32_t)context->sensorCount > cursor->limits->maxArrayElements )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_LIMIT_EXCEEDED );
		return;
	}
	for ( int32_t i = 0; i < context->sensorCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		int32_t shapeId = boxddReadI32( cursor );
		if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId, false ) )
		{
			return;
		}
		const uint8_t* shape = context->shapes + (size_t)shapeId * sizeof( b2Shape );
		if ( boxddRawI32( shape, offsetof( b2Shape, sensorIndex ) ) != i )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
		boxddValidateVisitorArray( context );
		boxddValidateVisitorArray( context );
		boxddValidateVisitorArray( context );
	}
	for ( int32_t i = 0; i < context->shapeCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		boxddSnapshotEntry* shapeEntry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, i );
		if ( shapeEntry == NULL || ( shapeEntry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		int32_t sensorIndex = boxddRawI32( context->shapes + (size_t)i * sizeof( b2Shape ), offsetof( b2Shape, sensorIndex ) );
		if ( !boxddValidIndex( sensorIndex, context->sensorCount ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
}

static void boxddParseIslands( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	int32_t islandCount = boxddReadI32( cursor );
	if ( islandCount != context->pools[5].nextIndex )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < islandCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		int32_t setIndex = boxddReadI32( cursor );
		int32_t localIndex = boxddReadI32( cursor );
		int32_t islandId = boxddReadI32( cursor );
		int32_t removeCount = boxddReadI32( cursor );
		int32_t bodyCount = 0;
		const uint8_t* bodies = boxddReadArray( cursor, &bodyCount, sizeof( int32_t ) );
		int32_t contactCount = 0;
		const uint8_t* contacts = boxddReadArray( cursor, &contactCount, sizeof( b2ContactLink ) );
		int32_t jointCount = 0;
		const uint8_t* joints = boxddReadArray( cursor, &jointCount, sizeof( b2JointLink ) );
		if ( cursor->status != BOXDD_SNAPSHOT_OK || !boxddValidatePoolSlot( context, BOXDD_SNAPSHOT_ENTRY_ISLAND, i, islandId ) )
		{
			return;
		}
		boxddSnapshotEntry* entry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_ISLAND, i );
		if ( entry == NULL || ( entry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			if ( islandId == BOXDD_NULL_INDEX && ( bodyCount != 0 || contactCount != 0 || jointCount != 0 ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			}
			continue;
		}
		entry->setIndex = setIndex;
		entry->localIndex = localIndex;
		if ( removeCount < 0 || localIndex < 0 || !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SOLVER_SET, setIndex, false ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
		for ( int32_t j = 0; j < bodyCount; ++j )
		{
			int32_t bodyId = boxddRawI32( bodies, (size_t)j * sizeof( int32_t ) );
			boxddSnapshotEntry* body = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyId );
			if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyId, false ) ||
				 ( body != NULL && body->ownerA != islandId ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
		}
		for ( int32_t j = 0; j < contactCount; ++j )
		{
			const uint8_t* link = contacts + (size_t)j * sizeof( b2ContactLink );
			int32_t contactId = boxddRawI32( link, offsetof( b2ContactLink, contactId ) );
			if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_CONTACT, contactId, false ) )
			{
				return;
			}
		}
		for ( int32_t j = 0; j < jointCount; ++j )
		{
			const uint8_t* link = joints + (size_t)j * sizeof( b2JointLink );
			int32_t jointId = boxddRawI32( link, offsetof( b2JointLink, jointId ) );
			if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_JOINT, jointId, false ) )
			{
				return;
			}
		}
	}
}

static bool boxddValidateTree( boxddSnapshotContext* context, int treeIndex )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	int32_t root = boxddReadI32( cursor );
	int32_t nodeCount = boxddReadI32( cursor );
	int32_t capacity = boxddReadI32( cursor );
	int32_t freeList = boxddReadI32( cursor );
	int32_t proxyCount = boxddReadI32( cursor );
	if ( capacity < 0 || (uint32_t)capacity > cursor->limits->maxTreeNodes || nodeCount < 0 || nodeCount > capacity || proxyCount < 0 ||
		 proxyCount > nodeCount || !boxddValidIndex( root, capacity ) || !boxddValidIndex( freeList, capacity ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return false;
	}
	size_t bytes = 0;
	if ( !boxddArrayBytes( cursor, (uint32_t)capacity, sizeof( b2TreeNode ), &bytes ) )
	{
		return false;
	}
	const uint8_t* nodes = boxddTake( cursor, bytes );
	if ( nodes == NULL )
	{
		return false;
	}
	context->treeNodes[treeIndex] = nodes;
	context->treeCapacities[treeIndex] = capacity;
	int32_t allocated = 0;
	int32_t leaves = 0;
	for ( int32_t i = 0; i < capacity; ++i )
	{
		const uint8_t* node = nodes + (size_t)i * sizeof( b2TreeNode );
		uint16_t flags = boxddRawU16( node, offsetof( b2TreeNode, flags ) );
		if ( ( flags & ~( b2_allocatedNode | b2_enlargedNode | b2_leafNode ) ) != 0 )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return false;
		}
		if ( ( flags & b2_allocatedNode ) == 0 )
		{
			if ( flags != 0 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
				return false;
			}
			int32_t next = boxddRawI32( node, offsetof( b2TreeNode, next ) );
			if ( !boxddValidIndex( next, capacity ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
			continue;
		}
		allocated += 1;
		if ( !boxddValidAabbBytes( node + offsetof( b2TreeNode, aabb ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			return false;
		}
		int32_t parent = boxddRawI32( node, offsetof( b2TreeNode, parent ) );
		if ( ( i == root && parent != BOXDD_NULL_INDEX ) || ( i != root && ( parent < 0 || parent >= capacity ) ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return false;
		}
		if ( i != root )
		{
			const uint8_t* parentNode = nodes + (size_t)parent * sizeof( b2TreeNode );
			uint16_t parentFlags = boxddRawU16( parentNode, offsetof( b2TreeNode, flags ) );
			int32_t parentChild1 =
				boxddRawI32( parentNode, offsetof( b2TreeNode, children ) + offsetof( b2TreeNodeChildren, child1 ) );
			int32_t parentChild2 =
				boxddRawI32( parentNode, offsetof( b2TreeNode, children ) + offsetof( b2TreeNodeChildren, child2 ) );
			if ( ( parentFlags & b2_allocatedNode ) == 0 || ( parentFlags & b2_leafNode ) != 0 ||
				 ( parentChild1 != i && parentChild2 != i ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
		}
		if ( ( flags & b2_leafNode ) != 0 )
		{
			if ( boxddRawU16( node, offsetof( b2TreeNode, height ) ) != 0 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
				return false;
			}
			leaves += 1;
			uint64_t userData = boxddRawU64( node, offsetof( b2TreeNode, userData ) );
			if ( userData > INT32_MAX )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
			int32_t shapeId = (int32_t)userData;
			if ( !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId, false ) )
			{
				return false;
			}
			const uint8_t* shape = context->shapes + (size_t)shapeId * sizeof( b2Shape );
			int32_t expectedProxy = ( i << 2 ) | treeIndex;
			boxddSnapshotEntry* bodyEntry =
				boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_BODY, boxddRawI32( shape, offsetof( b2Shape, bodyId ) ) );
			if ( boxddRawI32( shape, offsetof( b2Shape, proxyKey ) ) != expectedProxy ||
				 ( bodyEntry != NULL && bodyEntry->subtype != (uint32_t)treeIndex ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
		}
		else
		{
			int32_t child1 = boxddRawI32( node, offsetof( b2TreeNode, children ) + offsetof( b2TreeNodeChildren, child1 ) );
			int32_t child2 = boxddRawI32( node, offsetof( b2TreeNode, children ) + offsetof( b2TreeNodeChildren, child2 ) );
			if ( child1 == child2 || child1 == i || child2 == i || child1 < 0 || child1 >= capacity || child2 < 0 ||
				 child2 >= capacity )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
			const uint8_t* childNode1 = nodes + (size_t)child1 * sizeof( b2TreeNode );
			const uint8_t* childNode2 = nodes + (size_t)child2 * sizeof( b2TreeNode );
			if ( ( boxddRawU16( childNode1, offsetof( b2TreeNode, flags ) ) & b2_allocatedNode ) == 0 ||
				 ( boxddRawU16( childNode2, offsetof( b2TreeNode, flags ) ) & b2_allocatedNode ) == 0 ||
				 boxddRawI32( childNode1, offsetof( b2TreeNode, parent ) ) != i ||
				 boxddRawI32( childNode2, offsetof( b2TreeNode, parent ) ) != i )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
			uint16_t childHeight1 = boxddRawU16( childNode1, offsetof( b2TreeNode, height ) );
			uint16_t childHeight2 = boxddRawU16( childNode2, offsetof( b2TreeNode, height ) );
			uint32_t expectedHeight = 1u + ( childHeight1 > childHeight2 ? childHeight1 : childHeight2 );
			uint16_t childFlags = boxddRawU16( childNode1, offsetof( b2TreeNode, flags ) ) |
								  boxddRawU16( childNode2, offsetof( b2TreeNode, flags ) );
			if ( expectedHeight > UINT16_MAX || boxddRawU16( node, offsetof( b2TreeNode, height ) ) != expectedHeight ||
				 ( ( childFlags & b2_enlargedNode ) != 0 && ( flags & b2_enlargedNode ) == 0 ) ||
				 boxddRawU64( node, offsetof( b2TreeNode, categoryBits ) ) !=
					 ( boxddRawU64( childNode1, offsetof( b2TreeNode, categoryBits ) ) |
					   boxddRawU64( childNode2, offsetof( b2TreeNode, categoryBits ) ) ) ||
				 !boxddAabbContainsBytes( node + offsetof( b2TreeNode, aabb ), childNode1 + offsetof( b2TreeNode, aabb ) ) ||
				 !boxddAabbContainsBytes( node + offsetof( b2TreeNode, aabb ), childNode2 + offsetof( b2TreeNode, aabb ) ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
				return false;
			}
		}
	}
	if ( allocated != nodeCount || leaves != proxyCount || ( nodeCount == 0 ) != ( root == BOXDD_NULL_INDEX ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	if ( root != BOXDD_NULL_INDEX &&
		 ( boxddRawU16( nodes + (size_t)root * sizeof( b2TreeNode ), offsetof( b2TreeNode, flags ) ) & b2_allocatedNode ) == 0 )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	int32_t freeSeen = 0;
	int32_t current = freeList;
	while ( current != BOXDD_NULL_INDEX )
	{
		if ( current < 0 || current >= capacity || freeSeen > capacity )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return false;
		}
		const uint8_t* node = nodes + (size_t)current * sizeof( b2TreeNode );
		if ( ( boxddRawU16( node, offsetof( b2TreeNode, flags ) ) & b2_allocatedNode ) != 0 )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return false;
		}
		current = boxddRawI32( node, offsetof( b2TreeNode, next ) );
		freeSeen += 1;
	}
	if ( freeSeen != capacity - nodeCount )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return false;
	}
	for ( int32_t i = 0; i < capacity; ++i )
	{
		const uint8_t* node = nodes + (size_t)i * sizeof( b2TreeNode );
		if ( ( boxddRawU16( node, offsetof( b2TreeNode, flags ) ) & b2_allocatedNode ) == 0 )
		{
			continue;
		}
		int32_t walk = i;
		for ( int32_t depth = 0; depth <= capacity; ++depth )
		{
			if ( walk == root )
			{
				break;
			}
			if ( depth == capacity || !boxddCharge( cursor, 1 ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
			walk = boxddRawI32( nodes + (size_t)walk * sizeof( b2TreeNode ), offsetof( b2TreeNode, parent ) );
			if ( walk < 0 || walk >= capacity )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return false;
			}
		}
	}
	return true;
}

static void boxddParseBitset( boxddSnapshotCursor* cursor, int32_t maximumBits )
{
	uint32_t blocks = boxddReadU32( cursor );
	if ( blocks > cursor->limits->maxBitsetBlocks )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_LIMIT_EXCEEDED );
		return;
	}
	size_t bytes = 0;
	if ( !boxddArrayBytes( cursor, blocks, sizeof( uint64_t ), &bytes ) )
	{
		return;
	}
	const uint8_t* bits = boxddTake( cursor, bytes );
	if ( bits == NULL || blocks == 0 || maximumBits < 0 )
	{
		return;
	}
	uint32_t usedBits = (uint32_t)maximumBits & 63u;
	uint32_t neededBlocks = ( (uint32_t)maximumBits + 63u ) / 64u;
	if ( blocks > neededBlocks )
	{
		for ( uint32_t i = neededBlocks; i < blocks; ++i )
		{
			if ( boxddRawU64( bits, (size_t)i * sizeof( uint64_t ) ) != 0 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
		}
	}
	if ( usedBits != 0 && neededBlocks > 0 && blocks >= neededBlocks )
	{
		uint64_t last = boxddRawU64( bits, (size_t)( neededBlocks - 1u ) * sizeof( uint64_t ) );
		if ( ( last >> usedBits ) != 0 )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		}
	}
}

static uint64_t boxddKeyHash( uint64_t key )
{
	uint64_t hash = key;
	hash ^= hash >> 33;
	hash *= 0xff51afd7ed558ccdull;
	hash ^= hash >> 33;
	hash *= 0xc4ceb9fe1a85ec53ull;
	hash ^= hash >> 33;
	return hash;
}

static bool boxddHashContains( boxddSnapshotCursor* cursor, const uint8_t* items, uint32_t capacity, uint64_t key )
{
	uint32_t index = (uint32_t)boxddKeyHash( key ) & ( capacity - 1u );
	for ( uint32_t probes = 0; probes < capacity; ++probes )
	{
		if ( !boxddCharge( cursor, 1 ) )
		{
			return false;
		}
		uint64_t observed = boxddRawU64( items, (size_t)index * sizeof( b2SetItem ) );
		if ( observed == key )
		{
			return true;
		}
		if ( observed == 0 )
		{
			return false;
		}
		index = ( index + 1u ) & ( capacity - 1u );
	}
	return false;
}

static void boxddParseBroadPhase( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	for ( int tree = 0; tree < 3 && cursor->status == BOXDD_SNAPSHOT_OK; ++tree )
	{
		boxddValidateTree( context, tree );
	}
	for ( int tree = 0; tree < 3 && cursor->status == BOXDD_SNAPSHOT_OK; ++tree )
	{
		boxddParseBitset( cursor, context->treeCapacities[tree] );
	}
	int32_t moveCount = 0;
	const uint8_t* moves = boxddReadArray( cursor, &moveCount, sizeof( int32_t ) );
	for ( int32_t i = 0; i < moveCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
	{
		int32_t key = boxddRawI32( moves, (size_t)i * sizeof( int32_t ) );
		int32_t tree = key & 3;
		int32_t node = key >> 2;
		if ( tree < 0 || tree >= 3 || node < 0 || node >= context->treeCapacities[tree] )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
	uint32_t capacity = boxddReadU32( cursor );
	uint32_t count = boxddReadU32( cursor );
	if ( capacity > cursor->limits->maxHashCapacity || count > capacity || ( capacity != 0 && ( capacity & ( capacity - 1u ) ) != 0 ) ||
		 ( capacity == 0 && count != 0 ) )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
		return;
	}
	size_t bytes = 0;
	if ( !boxddArrayBytes( cursor, capacity, sizeof( b2SetItem ), &bytes ) )
	{
		return;
	}
	const uint8_t* items = boxddTake( cursor, bytes );
	if ( items == NULL )
	{
		return;
	}
	uint32_t occupied = 0;
	for ( uint32_t i = 0; i < capacity; ++i )
	{
		uint64_t key = boxddRawU64( items, (size_t)i * sizeof( b2SetItem ) );
		if ( key == 0 )
		{
			continue;
		}
		occupied += 1;
		int32_t shapeA = (int32_t)( key >> 32 );
		int32_t shapeB = (int32_t)( key & UINT32_MAX );
		if ( shapeA == shapeB || !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeA, false ) ||
			 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeB, false ) || !boxddHashContains( cursor, items, capacity, key ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
	if ( occupied != count )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	uint32_t liveContacts = context->facts.poolNext[3] - context->facts.poolFree[3];
	if ( count != liveContacts )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
		return;
	}
	for ( int32_t i = 0; i < context->contactCount; ++i )
	{
		boxddSnapshotEntry* contact = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_CONTACT, i );
		if ( contact == NULL || ( contact->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		uint32_t a = (uint32_t)contact->ownerA;
		uint32_t b = (uint32_t)contact->ownerB;
		uint64_t key = a < b ? ( (uint64_t)a << 32 ) | b : ( (uint64_t)b << 32 ) | a;
		if ( capacity == 0 || !boxddHashContains( cursor, items, capacity, key ) )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
}

static void boxddParseConstraintGraph( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	for ( int color = 0; color < BOXDD_GRAPH_COLOR_COUNT && cursor->status == BOXDD_SNAPSHOT_OK; ++color )
	{
		if ( color != BOXDD_OVERFLOW_COLOR )
		{
			boxddParseBitset( cursor, context->bodyCount );
		}
		int32_t contactCount = 0;
		const uint8_t* contacts = boxddReadArray( cursor, &contactCount, sizeof( b2ContactSim ) );
		int32_t jointCount = 0;
		const uint8_t* joints = boxddReadArray( cursor, &jointCount, sizeof( b2JointSim ) );
		for ( int32_t i = 0; i < contactCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
		{
			const uint8_t* sim = contacts + (size_t)i * sizeof( b2ContactSim );
			if ( !boxddValidateContactSimRelation( context, cursor, sim, b2_awakeSet, color, i ) )
			{
				return;
			}
			if ( !boxddValidateContactSimBools( cursor, sim ) )
			{
				return;
			}
			int32_t pointCount = boxddRawI32( sim, offsetof( b2ContactSim, manifold ) + offsetof( b2Manifold, pointCount ) );
			if ( pointCount < 0 || pointCount > 2 )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_VALUE );
			}
		}
		for ( int32_t i = 0; i < jointCount && cursor->status == BOXDD_SNAPSHOT_OK; ++i )
		{
			const uint8_t* sim = joints + (size_t)i * sizeof( b2JointSim );
			if ( !boxddValidateJointSimRelation( context, cursor, sim, b2_awakeSet, color, i ) )
			{
				return;
			}
			if ( !boxddValidateJointSimBools( cursor, sim ) )
			{
				return;
			}
		}
	}
}

static void boxddValidateShapeLists( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	for ( int32_t bodyId = 0; bodyId < context->bodyCount && cursor->status == BOXDD_SNAPSHOT_OK; ++bodyId )
	{
		boxddSnapshotEntry* bodyEntry = boxddEntry( context, BOXDD_SNAPSHOT_ENTRY_BODY, bodyId );
		if ( bodyEntry == NULL || ( bodyEntry->flags & BOXDD_SNAPSHOT_ENTRY_LIVE ) == 0 )
		{
			continue;
		}
		const uint8_t* body = context->bodies + (size_t)bodyId * sizeof( b2Body );
		int32_t expectedCount = boxddRawI32( body, offsetof( b2Body, shapeCount ) );
		int32_t shapeId = boxddRawI32( body, offsetof( b2Body, headShapeId ) );
		int32_t previous = BOXDD_NULL_INDEX;
		int32_t observed = 0;
		while ( shapeId != BOXDD_NULL_INDEX )
		{
			if ( observed > context->shapeCount || !boxddCharge( cursor, 1 ) ||
				 !boxddValidateReference( context, BOXDD_SNAPSHOT_ENTRY_SHAPE, shapeId, false ) )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
			const uint8_t* shape = context->shapes + (size_t)shapeId * sizeof( b2Shape );
			if ( boxddRawI32( shape, offsetof( b2Shape, bodyId ) ) != bodyId ||
				 boxddRawI32( shape, offsetof( b2Shape, prevShapeId ) ) != previous )
			{
				boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
				return;
			}
			previous = shapeId;
			shapeId = boxddRawI32( shape, offsetof( b2Shape, nextShapeId ) );
			observed += 1;
		}
		if ( observed != expectedCount )
		{
			boxddFail( cursor, BOXDD_SNAPSHOT_INVALID_REFERENCE );
			return;
		}
	}
}

static void boxddParseImage( boxddSnapshotContext* context )
{
	boxddSnapshotCursor* cursor = &context->cursor;
	const uint8_t* header = boxddTake( cursor, 16 );
	if ( header == NULL )
	{
		return;
	}
	uint32_t flags = boxddRawU32( header, 12 );
	uint32_t expectedFlags = ( B2_ENABLE_VALIDATION ? BOXDD_SNAPSHOT_VALIDATION_FLAG : 0u ) |
							 ( b2IsDoublePrecision() ? BOXDD_SNAPSHOT_DOUBLE_FLAG : 0u );
	if ( boxddRawU32( header, 0 ) != BOXDD_SNAPSHOT_MAGIC || boxddRawU32( header, 4 ) != BOXDD_SNAPSHOT_VERSION )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_BAD_HEADER );
		return;
	}
	if ( ( flags & ~BOXDD_SNAPSHOT_KNOWN_FLAGS ) != 0 || flags != expectedFlags ||
		 boxddRawU32( header, 8 ) != boxddAdapter_GetSnapshotLayoutHash() )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_ABI_MISMATCH );
		return;
	}
	context->facts.snapshotFlags = flags;
	boxddParseWorldConfig( context );
	boxddParsePools( context );
	context->solverOffset = cursor->cursor;
	boxddParseSolverSets( context, cursor, false );
	boxddParseBodies( context );
	boxddParseShapes( context );
	boxddParseContacts( context );
	boxddParseJoints( context );
	boxddParseChains( context );
	boxddParseSensors( context );
	boxddParseIslands( context );
	boxddParseBroadPhase( context );
	boxddParseConstraintGraph( context );
	if ( cursor->status != BOXDD_SNAPSHOT_OK )
	{
		return;
	}
	if ( cursor->cursor != cursor->size )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_TRAILING_BYTES );
		return;
	}
	if ( context->entries == NULL || context->entryCapacity < context->facts.requiredEntries )
	{
		boxddFail( cursor, BOXDD_SNAPSHOT_BUFFER_TOO_SMALL );
		return;
	}
	boxddSnapshotCursor solverCursor = *cursor;
	solverCursor.cursor = context->solverOffset;
	solverCursor.status = BOXDD_SNAPSHOT_OK;
	boxddParseSolverSets( context, &solverCursor, true );
	if ( solverCursor.status != BOXDD_SNAPSHOT_OK )
	{
		boxddFail( cursor, solverCursor.status );
		return;
	}
	cursor->work = solverCursor.work;
	boxddValidateShapeLists( context );
}

boxddSnapshotStatus boxddSnapshot_Validate( const uint8_t* image, size_t size, const boxddSnapshotLimits* limits,
										boxddSnapshotFacts* facts, boxddSnapshotEntry* entries, size_t entryCapacity,
										size_t* requiredEntries )
{
	if ( requiredEntries != NULL )
	{
		*requiredEntries = 0;
	}
	if ( facts != NULL )
	{
		memset( facts, 0, sizeof( *facts ) );
	}
	if ( image == NULL )
	{
		return BOXDD_SNAPSHOT_NULL_INPUT;
	}
	const boxddSnapshotLimits* selectedLimits = limits != NULL ? limits : &boxddDefaultLimits;
	boxddSnapshotStatus limitStatus = boxddValidateLimits( selectedLimits );
	if ( limitStatus != BOXDD_SNAPSHOT_OK )
	{
		return limitStatus;
	}
	if ( size > selectedLimits->maxImageBytes )
	{
		return BOXDD_SNAPSHOT_LIMIT_EXCEEDED;
	}
	if ( entryCapacity > 0 && entries == NULL )
	{
		return BOXDD_SNAPSHOT_INVALID_VALUE;
	}
	const uint16_t endianProbe = 1u;
	if ( *(const uint8_t*)&endianProbe != 1u )
	{
		return BOXDD_SNAPSHOT_ABI_MISMATCH;
	}

	boxddSnapshotContext context = { 0 };
	context.cursor = (boxddSnapshotCursor){ image, size, 0, 0, BOXDD_SNAPSHOT_OK, selectedLimits };
	context.entries = entries;
	context.entryCapacity = entryCapacity;
	context.facts.structSize = (uint32_t)sizeof( context.facts );
	context.facts.version = BOXDD_SNAPSHOT_FACTS_VERSION;
	context.facts.imageBytes = size;
	boxddParseImage( &context );
	context.facts.consumedBytes = context.cursor.cursor;
	context.facts.validationWork = context.cursor.work;
	if ( requiredEntries != NULL )
	{
		if ( context.facts.requiredEntries > SIZE_MAX )
		{
			return BOXDD_SNAPSHOT_OVERFLOW;
		}
		*requiredEntries = (size_t)context.facts.requiredEntries;
	}
	if ( facts != NULL )
	{
		*facts = context.facts;
	}
	return context.cursor.status;
}
