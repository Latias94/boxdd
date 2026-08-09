#if defined(BOXDD_ABI_SELECTED_DOUBLE)
#define BOXDD_ABI_OPPOSITE_PRECISION false
#else
#define BOX2D_DOUBLE_PRECISION 1
#define BOXDD_ABI_OPPOSITE_PRECISION true
#endif

#include <box2d/box2d.h>

bool boxdd_abi_probe_mixed_precision_matches(void)
{
	return b2IsDoublePrecision() == BOXDD_ABI_OPPOSITE_PRECISION;
}
