// SPDX-License-Identifier: MIT OR Apache-2.0

#include "boxdd_adapter.h"

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

#include <errno.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#ifndef BOXDD_UPSTREAM_SHA
#define BOXDD_UPSTREAM_SHA "unknown"
#endif

#ifndef BOXDD_ADAPTER_SOURCE_SHA256
#define BOXDD_ADAPTER_SOURCE_SHA256 "unknown"
#endif

#ifndef BOXDD_EFFECTIVE_SOURCE_SHA256
#error "BOXDD_EFFECTIVE_SOURCE_SHA256 must be supplied by the verified provider build"
#endif

#ifndef BOXDD_TARGET_ABI
#define BOXDD_TARGET_ABI "unknown"
#endif

#ifndef BOXDD_RECORDING_CONTRACT_BLAKE3
#error "BOXDD_RECORDING_CONTRACT_BLAKE3 must be supplied by the verified provider build"
#endif

#define BOXDD_SNAPSHOT_VERSION 3u
#define BOXDD_RECORDING_VERSION_MAJOR 3u
#define BOXDD_RECORDING_VERSION_MINOR 2u

_Static_assert( sizeof( BOXDD_EFFECTIVE_SOURCE_SHA256 ) == 65,
				"BOXDD_EFFECTIVE_SOURCE_SHA256 must be a 64-character digest" );
BOX2D_EXPORT const char boxddEffectiveSourceSha256[65] = BOXDD_EFFECTIVE_SOURCE_SHA256;

static void boxddMix64( uint64_t state[4], uint64_t value )
{
	static const uint64_t primes[4] = { 1099511628211ull, 14029467366897019727ull, 1609587929392839161ull,
									 9650029242287828579ull };
	for ( int i = 0; i < 4; ++i )
	{
		state[i] ^= value + (uint64_t)i * 0x9E3779B97F4A7C15ull;
		state[i] *= primes[i];
		state[i] ^= state[i] >> 29;
	}
}

static void boxddComputePrivateAbiHash( uint8_t out[32] )
{
	uint64_t state[4] = { 14695981039346656037ull, 0x6A09E667F3BCC909ull, 0xBB67AE8584CAA73Bull,
							 0x3C6EF372FE94F82Bull };
#define BOXDD_ABI_TYPE( type )                                                                                                    \
	boxddMix64( state, sizeof( type ) );                                                                                            \
	boxddMix64( state, _Alignof( type ) );
#define BOXDD_ABI_FIELD( type, field ) boxddMix64( state, offsetof( type, field ) );
#define BOXDD_ABI_VALUE( value ) boxddMix64( state, value );
#include "boxdd_private_abi.inl"
#undef BOXDD_ABI_TYPE
#undef BOXDD_ABI_FIELD
#undef BOXDD_ABI_VALUE

	for ( int i = 0; i < 4; ++i )
	{
		memcpy( out + 8 * i, state + i, 8 );
	}
}

uint32_t boxddAdapter_GetSnapshotLayoutHash( void )
{
	uint32_t hash = 2166136261u;
#define BOXDD_LAYOUT_VALUE( value )                                                                                               \
	do                                                                                                                              \
	{                                                                                                                               \
		hash ^= (uint32_t)( value );                                                                                                  \
		hash *= 16777619u;                                                                                                            \
	} while ( 0 );
#include "boxdd_snapshot_layout.inl"
#undef BOXDD_LAYOUT_VALUE
	return hash;
}

uint32_t boxddAdapter_AbiVersion( void )
{
	return BOXDD_ADAPTER_ABI_VERSION;
}

bool boxddAdapter_GetIdentity( boxddAdapterIdentity* out, size_t outSize )
{
	if ( out == NULL || outSize < sizeof( *out ) )
	{
		return false;
	}

	memset( out, 0, sizeof( *out ) );
	out->structSize = (uint32_t)sizeof( *out );
	out->abiVersion = BOXDD_ADAPTER_ABI_VERSION;
	out->snapshotVersion = BOXDD_SNAPSHOT_VERSION;
	out->recordingVersionMajor = BOXDD_RECORDING_VERSION_MAJOR;
	out->recordingVersionMinor = BOXDD_RECORDING_VERSION_MINOR;
	out->snapshotLayoutHash = boxddAdapter_GetSnapshotLayoutHash();
	out->pointerWidth = (uint8_t)sizeof( void* );
	const uint16_t endianProbe = 1u;
	out->littleEndian = *(const uint8_t*)&endianProbe;
	out->doublePrecision = b2IsDoublePrecision() ? 1u : 0u;
	out->validationEnabled = B2_ENABLE_VALIDATION ? 1u : 0u;
	boxddComputePrivateAbiHash( out->privateAbiHash );
	strncpy( out->upstreamSha, BOXDD_UPSTREAM_SHA, sizeof( out->upstreamSha ) - 1 );
	strncpy( out->targetAbi, BOXDD_TARGET_ABI, sizeof( out->targetAbi ) - 1 );
	strncpy( out->adapterSourceSha256, BOXDD_ADAPTER_SOURCE_SHA256, sizeof( out->adapterSourceSha256 ) - 1 );
	memcpy( out->effectiveSourceSha256, boxddEffectiveSourceSha256, sizeof( out->effectiveSourceSha256 ) );
	strncpy( out->recordingContractBlake3, BOXDD_RECORDING_CONTRACT_BLAKE3,
			 sizeof( out->recordingContractBlake3 ) - 1 );
	return true;
}

#if defined( __EMSCRIPTEN__ )
#ifndef BOXDD_WASM_PROVIDER_HEAP_LIMIT
#error "BOXDD_WASM_PROVIDER_HEAP_LIMIT must be supplied by the WASM provider build"
#endif

extern unsigned char __heap_base;

_Static_assert( ( _Alignof( max_align_t ) & ( _Alignof( max_align_t ) - 1 ) ) == 0,
				 "max_align_t alignment must be a power of two" );
_Static_assert( BOXDD_WASM_PROVIDER_HEAP_LIMIT > 0, "provider heap limit must be positive" );
_Static_assert( sizeof( uintptr_t ) <= sizeof( int64_t ), "provider heap coordinates must fit in sbrk64" );

static uintptr_t boxddWasmProgramBreak = (uintptr_t)&__heap_base;

// Emscripten's emmalloc normally expands to the full imported memory. The Rust module shares that
// memory and owns the upper partition, so provider sbrk must stop at the fixed partition boundary.
void* _sbrk64( int64_t increment )
{
	const uintptr_t alignment = _Alignof( max_align_t );
	const uintptr_t alignmentMask = alignment - 1;
	const uintptr_t heapBase = (uintptr_t)&__heap_base;
	const uintptr_t oldBreak = boxddWasmProgramBreak;

	if ( increment >= 0 )
	{
		const uint64_t requested = (uint64_t)increment;
		if ( requested > UINTPTR_MAX - alignmentMask )
		{
			errno = ENOMEM;
			return (void*)-1;
		}
		const uintptr_t rounded = ( (uintptr_t)requested + alignmentMask ) & ~alignmentMask;
		if ( oldBreak > BOXDD_WASM_PROVIDER_HEAP_LIMIT ||
			 rounded > BOXDD_WASM_PROVIDER_HEAP_LIMIT - oldBreak )
		{
			errno = ENOMEM;
			return (void*)-1;
		}
		boxddWasmProgramBreak = oldBreak + rounded;
		return (void*)oldBreak;
	}

	const uint64_t magnitude = (uint64_t)( -( increment + 1 ) ) + 1;
	const uintptr_t rounded = (uintptr_t)magnitude & ~alignmentMask;
	if ( magnitude > UINTPTR_MAX || oldBreak < heapBase || rounded > oldBreak - heapBase )
	{
		errno = ENOMEM;
		return (void*)-1;
	}
	boxddWasmProgramBreak = oldBreak - rounded;
	return (void*)oldBreak;
}

// Qualification-only export. It is intentionally absent from the public adapter header and Rust
// bindings; runtime smoke tests use it to prove that provider allocation cannot enter Rust memory.
uint32_t providerHeapBoundaryProbe( uintptr_t expectedLimit )
{
	const uintptr_t heapBase = (uintptr_t)&__heap_base;
	const uintptr_t initialBreak = boxddWasmProgramBreak;
	if ( expectedLimit != BOXDD_WASM_PROVIDER_HEAP_LIMIT || initialBreak < heapBase || initialBreak >= expectedLimit )
	{
		return 1;
	}

	const uintptr_t available = expectedLimit - initialBreak;
	if ( _sbrk64( (int64_t)available ) != (void*)initialBreak || boxddWasmProgramBreak != expectedLimit )
	{
		return 2;
	}

	errno = 0;
	void* overflow = _sbrk64( 1 );
	const int overflowErrno = errno;
	void* restored = _sbrk64( -(int64_t)available );
	if ( restored != (void*)expectedLimit || boxddWasmProgramBreak != initialBreak )
	{
		return 3;
	}
	return overflow == (void*)-1 && overflowErrno == ENOMEM ? 0 : 4;
}
#endif
