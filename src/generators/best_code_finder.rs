use super::CodeGenerator;
use crate::{Decoder, ErasureDecoder, ParityCheckMatrix, SimulationResult};
use rand::distributions::Standard;
use rand::{Rng, SeedableRng, thread_rng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

type CodeAndResult = (Option<ParityCheckMatrix>, SimulationResult);

/// An interface to find the best code generated by some code generator among a given number of
/// code.
/// 
/// # Example 
/// 
/// ```
/// use believer::BestCodeFinderUsingErasure;
/// use believer::RegularLDPCCodeGenerator;
/// 
/// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
/// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
///     .with_erasure_prob(0.5)
///     .among_n_codes(10);
/// let (code, result) = code_finder.find_best_code_simulating_n_iterations(1000);
/// ```
pub struct BestCodeFinderUsingErasure<'a, G: CodeGenerator> {
    code_generator: &'a G,
    erasure_prob: f64,
    n_codes_to_try: usize,
}

impl<'a, G: CodeGenerator> BestCodeFinderUsingErasure<'a, G> {
    // ***** Construction *****

    /// Creates a new `BestCodeFinderUsingErasure` from a given `code_generator`.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator);
    /// ```
    pub fn from_code_generator(code_generator: &'a G) -> Self {
        Self {
            code_generator,
            erasure_prob: 0.5,
            n_codes_to_try: 0,
        }
    }

    /// Set the number of codes to try for `self`.
    /// 
    /// If not specified, default to 0.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure
    ///     ::from_code_generator(&generator)
    ///     .among_n_codes(10);
    /// ```
    pub fn among_n_codes(mut self, n_codes: usize) -> Self {
        self.n_codes_to_try = n_codes;
        self
    }

    /// Set the erasure `prob` to use when simulating code performance. 
    /// 
    /// If not specified, default to 0.5.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure
    ///     ::from_code_generator(&generator)
    ///     .with_erasure_prob(0.2);
    /// ```
    pub fn with_erasure_prob(mut self, prob: f64) -> Self {
        if prob < 0.0 || prob > 1.0 {
            panic!("prob is not between 0 and 1")
        }
        self.erasure_prob = prob;
        self
    }

    /// Returns the best code and its performance obtained using the given random number generator 
    /// `rng`. 
    /// 
    /// To evaluate the performance of each code, `n_iterations` random error decoding are done.
    /// 
    /// It returns a pair of values. The first value is some code if at least one of the rate 
    /// bellow 1.0. If no code obtained better failure rate, none is return. The second element is
    /// the associated performance.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// use rand::thread_rng;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
    ///     .with_erasure_prob(0.5)
    ///     .among_n_codes(10);
    /// let (code, result) = code_finder
    ///     .find_best_code_simulating_n_iterations_with_rng(1000, &mut thread_rng());
    /// ```
    pub fn find_best_code_simulating_n_iterations_with_rng<R: Rng>(
        &self,
        n_iterations: usize,
        rng: &mut R,
    ) -> CodeAndResult {
        NIterationsBestCodeFinderUsingErasure::from(self)
            .with_n_iterations(n_iterations)
            .find_with_rng(rng)
    }

    /// Returns the best code and its performance obtained using the thread rng.
    /// 
    /// To evaluate the performance of each code, `n_iterations` random error decoding are done.
    /// 
    /// It returns a pair of values. The first value is some code if at least one of the rate 
    /// bellow 1.0. If not code obtained better failure rate, none is return. The second element is
    /// the associated performance.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
    ///     .with_erasure_prob(0.5)
    ///     .among_n_codes(10);
    /// let (code, result) = code_finder
    ///     .find_best_code_simulating_n_iterations(1000);
    /// ```
    pub fn find_best_code_simulating_n_iterations(&self, n_iterations: usize) -> CodeAndResult {
        self.find_best_code_simulating_n_iterations_with_rng(n_iterations, &mut thread_rng())
    }

    /// Returns the best code and its performance obtained using the given random number generator 
    /// `rng`. 
    /// 
    /// To evaluate the performance of each code, the code is simulated until `n_events` success
    /// and `n_events` failures. 
    /// 
    /// It returns a pair of values. The first value is some code if at least one of the rate 
    /// bellow 1.0. If no code obtained better failure rate, none is return. The second element is
    /// the associated performance.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// use rand::thread_rng;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
    ///     .with_erasure_prob(0.5)
    ///     .among_n_codes(10);
    /// let (code, result) = code_finder
    ///     .find_best_code_simulating_n_events_with_rng(25, &mut thread_rng());
    /// ```
    pub fn find_best_code_simulating_n_events_with_rng<R: Rng>(
        &self,
        n_events: usize,
        rng: &mut R,
    ) -> CodeAndResult {
        NEventsBestCodeFinderUsingErasure::from(self)
            .with_n_events(n_events)
            .find_with_rng(rng)
    }

    /// Returns the best code and its performance obtained using the thread rng.
    /// 
    /// To evaluate the performance of each code, the code is simulated until `n_events` success
    /// and `n_events` failures. 
    /// 
    /// It returns a pair of values. The first value is some code if at least one of the rate 
    /// bellow 1.0. If no code obtained better failure rate, none is return. The second element is
    /// the associated performance.
    /// 
    /// # Example 
    /// 
    /// ```
    /// use believer::BestCodeFinderUsingErasure;
    /// use believer::RegularLDPCCodeGenerator;
    /// 
    /// let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);
    /// let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
    ///     .with_erasure_prob(0.5)
    ///     .among_n_codes(10);
    /// let (code, result) = code_finder
    ///     .find_best_code_simulating_n_events(25);
    /// ```
    pub fn find_best_code_simulating_n_events(&self, n_events: usize) -> CodeAndResult {
        self.find_best_code_simulating_n_events_with_rng(n_events, &mut thread_rng())
    }
}

// The next 2 structs are basically the same things. They should be refactored.

struct NIterationsBestCodeFinderUsingErasure<'a, G: CodeGenerator> {
    code_finder: &'a BestCodeFinderUsingErasure<'a, G>,
    n_iterations: usize,
    random_seeds: Vec<u64>,
}

impl<'a, G: CodeGenerator> NIterationsBestCodeFinderUsingErasure<'a, G> {
    fn from(code_finder: &'a BestCodeFinderUsingErasure<'a, G>) -> Self {
        Self {
            code_finder,
            n_iterations: 0,
            random_seeds: Vec::new(),
        }
    }

    fn with_n_iterations(mut self, n_iterations: usize) -> Self {
        self.n_iterations = n_iterations;
        self
    }

    fn find_with_rng<R: Rng>(mut self, rng: &mut R) -> CodeAndResult {
        self.initialize_random_seeds_with_rng(rng);
        (0..self.code_finder.n_codes_to_try)
            .into_par_iter()
            .map(|code_index| {
                let mut rng = self.get_rng_for(code_index);
                self.simulate_one_code_with_rng(&mut rng)
            })
            .reduce(
                || (None, SimulationResult::worse_result()),
                |accumulator, code_and_result| Self::get_best_between(accumulator, code_and_result),
            )
    }

    fn initialize_random_seeds_with_rng<R: Rng>(&mut self, rng: &mut R) {
        self.random_seeds = rng
            .sample_iter(Standard)
            .take(self.code_finder.n_codes_to_try)
            .collect()
    }

    fn get_rng_for(&self, index: usize) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(self.random_seeds[index])
    }

    fn simulate_one_code_with_rng<R: Rng>(&self, rng: &mut R) -> CodeAndResult {
        let code = self.code_finder.code_generator.generate_with_rng(rng);
        let mut decoder = ErasureDecoder::with_prob(self.code_finder.erasure_prob).for_code(code);
        let result = decoder.simulate_n_iterations_with_rng(self.n_iterations, rng);
        (Some(decoder.take_code()), result)
    }

    fn get_best_between(first: CodeAndResult, second: CodeAndResult) -> CodeAndResult {
        if first.1.is_better_than(&second.1) {
            first
        } else {
            second
        }
    }
}

struct NEventsBestCodeFinderUsingErasure<'a, G: CodeGenerator> {
    code_finder: &'a BestCodeFinderUsingErasure<'a, G>,
    n_events: usize,
    random_seeds: Vec<u64>,
}

impl<'a, G: CodeGenerator> NEventsBestCodeFinderUsingErasure<'a, G> {
    fn from(code_finder: &'a BestCodeFinderUsingErasure<'a, G>) -> Self {
        Self {
            code_finder,
            n_events: 0,
            random_seeds: Vec::new(),
        }
    }

    fn with_n_events(mut self, n_events: usize) -> Self {
        self.n_events = n_events;
        self
    }

    fn find_with_rng<R: Rng>(mut self, rng: &mut R) -> CodeAndResult {
        self.initialize_random_seeds_with_rng(rng);
        (0..self.code_finder.n_codes_to_try)
            .into_par_iter()
            .map(|code_index| {
                let mut rng = self.get_rng_for(code_index);
                self.simulate_one_code_with_rng(&mut rng)
            })
            .reduce(
                || (None, SimulationResult::worse_result()),
                |accumulator, code_and_result| Self::get_best_between(accumulator, code_and_result),
            )
    }

    fn initialize_random_seeds_with_rng<R: Rng>(&mut self, rng: &mut R) {
        self.random_seeds = rng
            .sample_iter(Standard)
            .take(self.code_finder.n_codes_to_try)
            .collect()
    }

    fn get_rng_for(&self, index: usize) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(self.random_seeds[index])
    }

    fn simulate_one_code_with_rng<R: Rng>(&self, rng: &mut R) -> CodeAndResult {
        let code = self.code_finder.code_generator.generate_with_rng(rng);
        let mut decoder = ErasureDecoder::with_prob(self.code_finder.erasure_prob).for_code(code);
        let result = decoder.simulate_until_n_events_are_found_with_rng(self.n_events, rng);
        (Some(decoder.take_code()), result)
    }

    fn get_best_between(first: CodeAndResult, second: CodeAndResult) -> CodeAndResult {
        if first.1.is_better_than(&second.1) {
            first
        } else {
            second
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::RegularLDPCCodeGenerator;
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn reproductibility_for_finding_best_ldpc_code_simulating_n_iterations() {
        let rng = ChaCha8Rng::seed_from_u64(123);
        let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);

        let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
            .with_erasure_prob(0.25)
            .among_n_codes(10);

        let code_and_result_0 =
            code_finder.find_best_code_simulating_n_iterations_with_rng(50, &mut rng.clone());

        let code_and_result_1 =
            code_finder.find_best_code_simulating_n_iterations_with_rng(50, &mut rng.clone());
        assert_eq!(code_and_result_0, code_and_result_1);
    }

    #[test]
    fn reproductibility_for_finding_best_ldpc_code_simulating_n_events() {
        let rng = ChaCha8Rng::seed_from_u64(123);
        let generator = RegularLDPCCodeGenerator::new(3, 4, 2, 4);

        let code_finder = BestCodeFinderUsingErasure::from_code_generator(&generator)
            .with_erasure_prob(0.25)
            .among_n_codes(10);

        let code_and_result_0 = code_finder
            .find_best_code_simulating_n_events_with_rng(50, &mut rng.clone());
        let code_and_result_1 = code_finder
            .find_best_code_simulating_n_events_with_rng(50, &mut rng.clone());

        assert_eq!(code_and_result_0, code_and_result_1);
    }
}
