// SPDX-License-Identifier: MIT OR Apache-2.0

#include "boxdd_adapter.h"

#include "recording_replay.h"

bool boxddRecPlayer_IsHealthy( const b2RecPlayer* player )
{
	return player != NULL && player->rdr.ok;
}
