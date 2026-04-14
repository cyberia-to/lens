// ============================================================================
// tropical.wgsl — tropical semiring matrix multiplication for GPU compute
// ============================================================================
//
// The tropical semiring (min, +) over u32:
//   - Tropical addition: min(a, b)
//   - Tropical multiplication: saturating_add(a, b)
//   - Additive identity (ZERO): 0xFFFFFFFF (infinity)
//   - Multiplicative identity (ONE): 0
//
// Matrix multiplication:
//   C[i][j] = min_k (A[i][k] + B[k][j])
//
// where min is tropical addition and + is tropical multiplication
// (which is ordinary addition, saturating at u32::MAX).
//
// The shader implements tiled matrix multiplication with 16x16 workgroups.
// Each thread computes one element of the output matrix by iterating over
// tiles of the shared dimension.
//
// WGSL constraints:
//   - No u64 types; we use u32 (u32::MAX = infinity)
//   - Workgroup shared memory for tiled access
//   - Workgroup size 16x16 = 256 threads
// ============================================================================


// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Tropical infinity: the additive identity (neutral element for min).
const TROP_INF: u32 = 0xFFFFFFFFu;

// Tile dimension for shared memory blocking.
const TILE_DIM: u32 = 16u;


// ---------------------------------------------------------------------------
// Tropical element operations
// ---------------------------------------------------------------------------

/// Tropical addition: min(a, b).
/// If either operand is INF, returns the other.
fn trop_add(a: u32, b: u32) -> u32 {
    return min(a, b);
}

/// Tropical multiplication: a + b (saturating).
/// If either operand is INF, or if the sum overflows, returns INF.
fn trop_mul(a: u32, b: u32) -> u32 {
    if a == TROP_INF || b == TROP_INF {
        return TROP_INF;
    }
    let sum = a + b;
    // Check for overflow: if sum < a, it wrapped around.
    // Also check if sum equals TROP_INF (reserved for infinity).
    if sum < a || sum == TROP_INF {
        return TROP_INF;
    }
    return sum;
}


// ---------------------------------------------------------------------------
// Buffer layout
// ---------------------------------------------------------------------------
//
// Matrices are stored in row-major order as flat arrays of u32.
//   A: n x n matrix (input)
//   B: n x n matrix (input)
//   C: n x n matrix (output)
//
// params.n is the matrix dimension.

struct Params {
    n: u32,
}

@group(0) @binding(0) var<storage, read>       matrix_a: array<u32>;
@group(0) @binding(1) var<storage, read>       matrix_b: array<u32>;
@group(0) @binding(2) var<storage, read_write> matrix_c: array<u32>;
@group(0) @binding(3) var<uniform>             params:   Params;


// ---------------------------------------------------------------------------
// Shared memory for tiling
// ---------------------------------------------------------------------------

var<workgroup> tile_a: array<u32, 256>;  // 16 x 16
var<workgroup> tile_b: array<u32, 256>;  // 16 x 16


// ---------------------------------------------------------------------------
// Tiled tropical matrix multiplication
// ---------------------------------------------------------------------------
//
// Each workgroup computes a 16x16 tile of the output matrix C.
// The algorithm iterates over tiles along the shared dimension k:
//
//   for each tile t in 0..ceil(n/16):
//     1. Load tile_a[local_row][local_col] = A[global_row][t*16 + local_col]
//     2. Load tile_b[local_row][local_col] = B[t*16 + local_row][global_col]
//     3. Barrier
//     4. For each k in 0..16:
//          acc = trop_add(acc, trop_mul(tile_a[local_row][k], tile_b[k][local_col]))
//     5. Barrier
//
//   C[global_row][global_col] = acc
//
// Out-of-bounds accesses load TROP_INF, which is the neutral element for
// tropical addition (min), so they do not affect the result.

@compute @workgroup_size(16, 16)
fn tropical_matmul(@builtin(global_invocation_id) gid: vec3<u32>,
                   @builtin(local_invocation_id) lid: vec3<u32>) {
    let n = params.n;
    let row = gid.x;
    let col = gid.y;
    let local_row = lid.x;
    let local_col = lid.y;

    var acc: u32 = TROP_INF;

    let num_tiles = (n + TILE_DIM - 1u) / TILE_DIM;

    for (var t: u32 = 0u; t < num_tiles; t = t + 1u) {
        // Load A[row][t*TILE_DIM + local_col] into shared memory
        let a_col = t * TILE_DIM + local_col;
        if row < n && a_col < n {
            tile_a[local_row * TILE_DIM + local_col] = matrix_a[row * n + a_col];
        } else {
            tile_a[local_row * TILE_DIM + local_col] = TROP_INF;
        }

        // Load B[t*TILE_DIM + local_row][col] into shared memory
        let b_row = t * TILE_DIM + local_row;
        if b_row < n && col < n {
            tile_b[local_row * TILE_DIM + local_col] = matrix_b[b_row * n + col];
        } else {
            tile_b[local_row * TILE_DIM + local_col] = TROP_INF;
        }

        workgroupBarrier();

        // Accumulate: acc = min over k of (acc, A[row][k] + B[k][col])
        for (var k: u32 = 0u; k < TILE_DIM; k = k + 1u) {
            let a_val = tile_a[local_row * TILE_DIM + k];
            let b_val = tile_b[k * TILE_DIM + local_col];
            let product = trop_mul(a_val, b_val);
            acc = trop_add(acc, product);
        }

        workgroupBarrier();
    }

    // Write result
    if row < n && col < n {
        matrix_c[row * n + col] = acc;
    }
}
