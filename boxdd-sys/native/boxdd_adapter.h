// SPDX-License-Identifier: MIT OR Apache-2.0

#pragma once

#include "box2d/base.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

#define BOXDD_ADAPTER_ABI_VERSION 2u
#define BOXDD_SNAPSHOT_FACTS_VERSION 1u
#define BOXDD_SNAPSHOT_ENTRY_VERSION 1u

// Stable status values. These are uint32_t instead of a C enum so the FFI width is explicit.
typedef uint32_t boxddSnapshotStatus;
#define BOXDD_SNAPSHOT_OK 0u
#define BOXDD_SNAPSHOT_NULL_INPUT 1u
#define BOXDD_SNAPSHOT_TRUNCATED 2u
#define BOXDD_SNAPSHOT_BAD_HEADER 3u
#define BOXDD_SNAPSHOT_ABI_MISMATCH 4u
#define BOXDD_SNAPSHOT_OVERFLOW 5u
#define BOXDD_SNAPSHOT_LIMIT_EXCEEDED 6u
#define BOXDD_SNAPSHOT_INVALID_VALUE 7u
#define BOXDD_SNAPSHOT_INVALID_REFERENCE 8u
#define BOXDD_SNAPSHOT_DUPLICATE 9u
#define BOXDD_SNAPSHOT_TRAILING_BYTES 10u
#define BOXDD_SNAPSHOT_BUFFER_TOO_SMALL 11u

typedef uint32_t boxddSnapshotEntryKind;
#define BOXDD_SNAPSHOT_ENTRY_BODY 1u
#define BOXDD_SNAPSHOT_ENTRY_SHAPE 2u
#define BOXDD_SNAPSHOT_ENTRY_CHAIN 3u
#define BOXDD_SNAPSHOT_ENTRY_CONTACT 4u
#define BOXDD_SNAPSHOT_ENTRY_JOINT 5u
#define BOXDD_SNAPSHOT_ENTRY_ISLAND 6u
#define BOXDD_SNAPSHOT_ENTRY_SOLVER_SET 7u

#define BOXDD_SNAPSHOT_ENTRY_LIVE 0x00000001u
#define BOXDD_SNAPSHOT_ENTRY_REQUIRES_CUSTOM_FILTER 0x00000002u
#define BOXDD_SNAPSHOT_ENTRY_REQUIRES_PRE_SOLVE 0x00000004u

typedef struct boxddAdapterIdentity
{
	uint32_t structSize;
	uint32_t abiVersion;
	uint32_t snapshotVersion;
	uint32_t recordingVersionMajor;
	uint32_t recordingVersionMinor;
	uint32_t snapshotLayoutHash;
	uint8_t pointerWidth;
	uint8_t littleEndian;
	uint8_t doublePrecision;
	uint8_t validationEnabled;
	uint8_t privateAbiHash[32];
	char upstreamSha[41];
	char targetAbi[65];
	char adapterSourceSha256[65];
	char effectiveSourceSha256[65];
	char recordingContractBlake3[65];
} boxddAdapterIdentity;

typedef struct boxddSnapshotLimits
{
	uint32_t structSize;
	uint32_t version;
	uint64_t maxImageBytes;
	uint64_t maxValidationWork;
	uint32_t maxEntries;
	uint32_t maxArrayElements;
	uint32_t maxTreeNodes;
	uint32_t maxHashCapacity;
	uint32_t maxBitsetBlocks;
	uint32_t reserved;
} boxddSnapshotLimits;

typedef struct boxddSnapshotFacts
{
	uint32_t structSize;
	uint32_t version;
	uint64_t imageBytes;
	uint64_t consumedBytes;
	uint64_t requiredEntries;
	uint64_t validationWork;
	uint32_t snapshotFlags;
	uint32_t worldFlags;
	uint32_t poolNext[7];
	uint32_t poolFree[7];
	uint32_t entryCounts[7];
	uint32_t requiresCustomFilter;
	uint32_t requiresPreSolve;
} boxddSnapshotFacts;

typedef struct boxddSnapshotEntry
{
	uint32_t structSize;
	uint32_t version;
	uint32_t kind;
	uint32_t flags;
	int32_t index;
	int32_t ownerA;
	int32_t ownerB;
	int32_t setIndex;
	int32_t localIndex;
	int32_t colorIndex;
	int32_t freeOrder;
	uint32_t generation;
	uint32_t subtype;
	int32_t ownerAPrev;
	int32_t ownerANext;
	int32_t ownerBPrev;
	int32_t ownerBNext;
	int32_t ownerBOrder;
} boxddSnapshotEntry;

typedef struct b2RecPlayer b2RecPlayer;

B2_API uint32_t boxddAdapter_AbiVersion( void );
B2_API bool boxddAdapter_GetIdentity( boxddAdapterIdentity* out, size_t outSize );
B2_API uint32_t boxddAdapter_GetSnapshotLayoutHash( void );
extern BOX2D_EXPORT const char boxddEffectiveSourceSha256[65];

// A NULL entries pointer is a sizing pass. BUFFER_TOO_SMALL is not authorization to use the image;
// callers must run a second pass with at least facts.requiredEntries entries and require OK.
B2_API boxddSnapshotStatus boxddSnapshot_Validate( const uint8_t* image, size_t size, const boxddSnapshotLimits* limits,
											 boxddSnapshotFacts* facts, boxddSnapshotEntry* entries, size_t entryCapacity,
											 size_t* requiredEntries );

// The upstream public replay API conflates malformed input and clean EOF. This adapter exposes the
// private reader health bit without exposing the private player layout to Rust.
B2_API bool boxddRecPlayer_IsHealthy( const b2RecPlayer* player );

#ifdef __cplusplus
}
#endif
