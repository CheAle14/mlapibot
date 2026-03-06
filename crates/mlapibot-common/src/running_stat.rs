type Num = f32;

/// Calculates the maximum, mean, variance and standard deviation of a series
/// of values without keeping them all in memory.
///
/// Converted from C++: https://www.johndcook.com/blog/standard_deviation/
#[derive(Default)]
pub struct RunningStat {
    m_n: usize,
    m_old_m: Num,
    m_new_m: Num,
    m_old_s: Num,
    m_new_s: Num,

    max: Num,
}

impl RunningStat {
    pub fn clear(&mut self) {
        self.m_n = 0;
    }

    pub fn push(&mut self, value: Num) {
        self.m_n += 1;

        if value > self.max {
            self.max = value;
        }

        if self.m_n == 1 {
            self.m_new_m = value;
            self.m_old_m = self.m_new_m;
            self.m_old_s = 0.0;
        } else {
            self.m_new_m = self.m_old_m + (value - self.m_old_m) / (self.m_n as Num);
            self.m_new_s = self.m_old_s + (value - self.m_old_m) * (value - self.m_new_m);

            self.m_old_m = self.m_new_m;
            self.m_old_s = self.m_new_s;
        }
    }

    pub fn results(&self) -> DataMetaData {
        let mean = if self.m_n > 0 { self.m_new_m } else { 0.0 };

        let variance = if self.m_n > 1 {
            self.m_new_s / (self.m_n - 1) as Num
        } else {
            0.0
        };

        let standard_dev = variance.sqrt();

        DataMetaData {
            count: self.m_n,
            max: self.max,
            mean,
            variance,
            standard_dev,
        }
    }
}

impl std::ops::AddAssign<Num> for RunningStat {
    fn add_assign(&mut self, rhs: Num) {
        self.push(rhs);
    }
}

#[derive(Debug)]
pub struct DataMetaData {
    pub count: usize,
    pub mean: Num,
    pub variance: Num,
    pub standard_dev: Num,
    pub max: Num,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_stats() {
        let dist = vec![
            72.2, 23.0, 0.0, 50.0, 7.0, 51.0, 61.0, 85.0, 1.0, 7.0, 82.0, 65.0, 45.0, 7.0, 12.0,
            82.0, 32.0, 86.0, 79.0, 25.0,
        ];

        let mut stats = RunningStat::default();

        for s in &dist {
            stats += *s;
        }

        let true_mean = dist.iter().copied().sum::<Num>() / dist.len() as Num;
        let true_variance = dist
            .iter()
            .copied()
            .map(|v| square(v - true_mean))
            .sum::<Num>()
            / (dist.len() - 1) as Num;
        let true_std_dev = true_variance.sqrt();

        let meta = stats.results();

        assert!(
            (meta.mean - true_mean).abs() < 1.0,
            "{} approx {true_mean}",
            meta.mean
        );

        assert!(
            (meta.variance - true_variance).abs() < 1.0,
            "{} approx {true_variance}",
            meta.variance
        );

        assert!(
            (meta.standard_dev - true_std_dev).abs() < 1.0,
            "{} approx {true_std_dev}",
            meta.standard_dev
        );
    }

    fn square(num: Num) -> Num {
        num * num
    }
}
