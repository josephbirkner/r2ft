/*
 * copied&pasted and later modified from:
 * https://github.com/althonos/pruefung/blob/master/src/fnv/fnv32.rs
 * which is under MIT License.
 */

#[cfg(feature = "generic")]
extern crate digest;
#[cfg(feature = "generic")]
extern crate generic_array;

use crate::common::Cursor;
use byteorder::ReadBytesExt;
use core::hash::Hasher;
#[cfg(feature = "generic")]
use digest;
#[cfg(feature = "generic")]
use generic_array;
use std::io::{Read, Seek, Write};

const FNV_OFFSET: u32 = 0x811C9DC5;
const FNV_PRIME: u32 = 0x01000193;

/// Implement [`digest::Digest`][1] for a struct implementing [`hash::Hasher`][2].
///
/// [1]: https://docs.rs/digest/trait.Digest.html
/// [2]: https://doc.rust-lang.org/core/hash/trait.Hasher.html
#[allow(unused)]
macro_rules! implement_digest {
    ($Hasher:ident, $BlockSize:ident, $OutputSize:ident) => {
        #[cfg(feature = "generic")]
        impl digest::BlockInput for $Hasher {
            type BlockSize = digest::generic_array::typenum::$BlockSize;
        }

        #[cfg(feature = "generic")]
        impl digest::Input for $Hasher {
            #[inline]
            fn process(&mut self, input: &[u8]) {
                self.write(input)
            }
        }

        #[cfg(feature = "generic")]
        impl digest::FixedOutput for $Hasher {
            type OutputSize = digest::generic_array::typenum::$OutputSize;
            #[inline]
            fn fixed_result(self) -> generic_array::GenericArray<u8, Self::OutputSize> {
                use generic_array::typenum::Unsigned;
                let mut array = digest::generic_array::GenericArray::default();
                let mut out = self.finish();
                let size = Self::OutputSize::to_usize();
                for i in 0..size {
                    array[size - i - 1] = (out & u8::max_value() as u64) as u8;
                    out >>= 8;
                }
                array
            }
        }
    };
}

/// The FNV1a-32 hasher.
#[derive(Copy, Clone, Debug)]
pub struct Fnv32a {
    state: u32,
}

impl Fnv32a {
    /// return 32bit FNV1a hash of `cursor` between `from` until excluding `to`
    pub fn hash(cursor: &mut Cursor, from: u64, to: u64) -> u32 {
        let pos = cursor.position();

        let mut hasher = Fnv32a::default();
        let mut byte: [u8; 1] = [0];
        cursor.set_position(from);
        for _ in from..to {
            cursor.read_exact(&mut byte);
            hasher.write(&byte);
        }

        cursor.set_position(pos);
        return hasher.finish() as u32;
    }

    #[inline]
    fn write_u8(&mut self, byte: u8) {
        self.state ^= byte as u32;
        self.state = self.state.wrapping_mul(FNV_PRIME);
    }
}

impl Default for Fnv32a {
    fn default() -> Self {
        Fnv32a { state: FNV_OFFSET }
    }
}

impl Hasher for Fnv32a {
    #[inline]
    fn write(&mut self, input: &[u8]) {
        for &byte in input.iter() {
            self.write_u8(byte);
        }
    }

    // actually u32
    #[inline]
    fn finish(&self) -> u64 {
        self.state as u64
    }
}

implement_digest!(Fnv32a, U2048, U4);

mod test {

    use super::{Fnv32a, Hasher};

    fn test(a: u32, b: &[u8]) {
        println!("\nTesting {:?}.", String::from_utf8_lossy(b));
        let mut hasher = Fnv32a::default();
        hasher.write(b);
        println!("hash({:x?})", b);
        let r = hasher.finish();
        println!("Should: {:x}; Is: {:x}", a, r);
        assert_eq!(r, a as u64)
    }

    #[test]
    fn test_a() {
        test(0x811c9dc5, b"");
        test(0xd10c0b43, b"T");
        test(0x2f4ee094, b"The ");
        test(0x048fff90, b"The quick brown fox jumps over the lazy dog");
        test(0xe35a21cb, b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Pharetra sit amet aliquam id. Tortor at risus viverra adipiscing at in. Risus nec feugiat in fermentum posuere urna nec tincidunt praesent. Viverra accumsan in nisl nisi scelerisque eu ultrices vitae auctor. Blandit massa enim nec dui nunc mattis enim ut tellus. Eros in cursus turpis massa tincidunt. Nulla aliquet enim tortor at auctor. Purus semper eget duis at tellus at urna condimentum. Vitae suscipit tellus mauris a diam maecenas sed enim. Massa enim nec dui nunc mattis. Diam vel quam elementum pulvinar etiam non quam lacus suspendisse. Elementum nisi quis eleifend quam. Lacus vestibulum sed arcu non odio. Diam maecenas sed enim ut sem. Est sit amet facilisis magna etiam tempor. Tristique senectus et netus et malesuada fames ac.");
    }
}
