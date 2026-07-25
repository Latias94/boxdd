// SPDX-License-Identifier: MIT OR Apache-2.0

#include "bitset.h"
#include "body.h"
#include "broad_phase.h"
#include "constraint_graph.h"
#include "contact.h"
#include "id_pool.h"
#include "island.h"
#include "joint.h"
#include "recording.h"
#include "sensor.h"
#include "shape.h"
#include "solver_set.h"
#include "table.h"

#include "box2d/box2d.h"
#include "box2d/collision.h"
#include "box2d/types.h"

#include <stddef.h>
#include <stdint.h>

#define BOXDD_SNAPSHOT_VERSION 3u

// These symbols are consumed by build.rs from the target object. They deliberately contain only
// target-compiler constants and are not part of the public adapter ABI.
const uint64_t boxddPrivateAbiValues[] = {
#define BOXDD_ABI_TYPE( type ) (uint64_t)sizeof( type ), (uint64_t)_Alignof( type ),
#define BOXDD_ABI_FIELD( type, field ) (uint64_t)offsetof( type, field ),
#define BOXDD_ABI_VALUE( value ) (uint64_t)( value ),
#include "boxdd_private_abi.inl"
#undef BOXDD_ABI_TYPE
#undef BOXDD_ABI_FIELD
#undef BOXDD_ABI_VALUE
};
const uint64_t boxddPrivateAbiValueCount = sizeof( boxddPrivateAbiValues ) / sizeof( boxddPrivateAbiValues[0] );

const uint64_t boxddSnapshotLayoutValues[] = {
#define BOXDD_LAYOUT_VALUE( value ) (uint64_t)( value ),
#include "boxdd_snapshot_layout.inl"
#undef BOXDD_LAYOUT_VALUE
};
const uint64_t boxddSnapshotLayoutValueCount = sizeof( boxddSnapshotLayoutValues ) / sizeof( boxddSnapshotLayoutValues[0] );
