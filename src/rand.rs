use self::inner::ReseedingRng;
use base64ct::{Base64Url, Encoding as _};
use rand::rngs::{ChaCha20Rng, SysRng};
use rand::{RngExt as _, SeedableRng as _};
use std::cell::UnsafeCell;
use std::marker::PhantomData;

thread_local! {
	static RNG: ReseedingRng = ReseedingRng::new();
}

pub fn gen_auth_state() -> String {
	rand_768_bit_encoded_string()
}

pub fn gen_auth_token() -> String {
	rand_768_bit_encoded_string()
}

fn rand_768_bit_encoded_string() -> String {
	let mut bytes = [0u8; 96];
	RNG.with(|rng| rng.fill_bytes(&mut bytes));
	Base64Url::encode_string(&bytes)
}

mod inner {
	use super::*;

	pub(super) struct ReseedingRng {
		inner: UnsafeCell<ReseedingRngInner>,
		__not_thread_safe: PhantomData<*mut ()>,
	}

	struct ReseedingRngInner {
		rng: ChaCha20Rng,
		remaining: u64,
	}

	impl ReseedingRng {
		pub fn new() -> Self {
			Self {
				inner: UnsafeCell::new(ReseedingRngInner {
					rng: get_newly_seeded_rng(),
					remaining: RESEED_THRESHOLD,
				}),
				__not_thread_safe: PhantomData,
			}
		}

		pub fn fill_bytes(&self, bytes: &mut [u8]) {
			// SAFETY: we're fine .3
			// since we are explicitly not thread safe, only situation where this
			// isn't fine is if we somehow call this again and create a second
			// mutable reference while the first one is still being held. As long
			// as the reference is created, used, then dropped within a single
			// function body, and nothing creates another reference while we have
			// one, we're fine
			let rng = unsafe { &mut *self.inner.get() };
			rng.fill_bytes(bytes);
		}
	}

	impl ReseedingRngInner {
		pub fn fill_bytes(&mut self, bytes: &mut [u8]) {
			self.rng.fill(bytes);
			self.mark_bytes_generated_and_maybe_reseed(bytes.len() as _);
		}

		fn mark_bytes_generated_and_maybe_reseed(&mut self, bytes: u64) {
			self.remaining = self.remaining.saturating_sub(bytes);

			if self.remaining == 0 {
				self.remaining = RESEED_THRESHOLD;
				self.rng = get_newly_seeded_rng();
			}
		}
	}

	fn get_newly_seeded_rng() -> ChaCha20Rng {
		ChaCha20Rng::try_from_rng(&mut SysRng).expect("failed to seed rng")
	}

	const RESEED_THRESHOLD: u64 = 1 << 16;
}
