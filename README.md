# binary-greedy-meshing
Originally a port of [Binary Greedy Meshing v2](https://github.com/cgerikj/binary-greedy-meshing) to Rust, with additional improvements such as support for transparent blocks.

## How to use
This crate is used in the Bevy voxel game [Riverbed](https://github.com/Inspirateur/riverbed), you can check out the code for usage examples.

### Minimal example
```rust
use binary_greedy_meshing as bgm;
use std::collections::BTreeSet;

fn main() {
    // This is a flattened 3D array of u16 in ZXY order, of size 64^3 
    // (it represents a 62^3-sized chunk that is padded with neighbor information)
    let mut voxels = [0; bgm::CS_P3];
    // Add 2 voxels of value "1" at position 0;0;0 and 0;1;0
    voxels[bgm::pad_linearize(0, 0, 0)] = 1;
    voxels[bgm::pad_linearize(0, 1, 0)] = 1;
    // Add 1 voxel of value "2" at position 0;2;0
    voxels[bgm::pad_linearize(0, 1, 0)] = 2;
    // Say the value 2 is transparent
    let transparent_blocks = BTreeSet::from([2]);
    // Contain useful buffers that can be cached and cleared 
    // with mesh_data.clear() to avoid re-allocation
    let mut mesher = bgm::MeshData::new();
    // 2 methods are available for the meshing:
    // The "mesh" method only takes the voxel buffer and a BTreeSet signaling the transparent values
    // mesher.mesh(&voxels, transparent_blocks);
    // The "fast_mesh" method is ~4x faster
    // but requires maintaining an opacity and transparency mask for the chunk
    let opaque_mask = bgm::compute_opaque_mask(voxels.as_slice(), &transparent_blocks);
    let trans_mask = bgm::compute_transparent_mask(voxels.as_slice(), &transparent_blocks);
    mesher.fast_mesh(&voxels, &opaque_mask, &trans_mask);
    // Both methods have the same "output" which is stored in mesher.quads
}
```

### What to do with `mesh_data.quads`
`mesh_data.quads` is a `[Vec<u64>; 6]`, 1 Vec<u64> per face type, each u64 encoding all the information of a quad in the following manner:
```rust
(v_type << 32) | (h << 24) | (w << 18) | (z << 12) | (y << 6) | x
```

The face groups correspond to Up, Down, Right, Left, Front, Back, in this order. (assuming right handed Y up)

The fastest way of rendering quads is using instancing (check [this video](https://www.youtube.com/watch?v=40JzyaOYJeY) to learn more about the topic), but if it's not available you can still convert the quads to vertices and indices making a regular mesh, see this Riverbed files for an example of this:
- [src/render/mesh_utils.rs](https://github.com/Inspirateur/riverbed/blob/main/src/render/mesh_utils.rs) for Face+Quad => vertices conversion
- [src/render/mesh_chunks.rs](https://github.com/Inspirateur/riverbed/blob/main/src/render/mesh_chunks.rs) for the rest of the meshing code (+ LOD)

## Benchmarks
running `cargo bench` on AMD Ryzen 5 5500 3.60 GHz:
- "fast_mesh" with opaque voxels only: **65 µs**
- "mesh" with opaque voxels only: **300 µs**
- "fast_mesh" with opaque & transparents voxels: **90 µs**
- "mesh" with opaque & transparents voxels: **340 µs**

This is in line with the 50-200μs performance range reported from the original C version of the library  (which doesn't yet support transparency).

The meshing is also ~30x faster than [block-mesh-rs](https://github.com/bonsairobo/block-mesh-rs) which took **~3ms** to greedy mesh a chunk on my machine.

*chunk sizes are 62^3 (64^3 with padding), this crate doesn't support other sizes.*
