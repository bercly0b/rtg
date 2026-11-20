//! Registration estimate: RTG's own guess at when an account was registered,
//! derived from its user ID.
//!
//! Telegram handed out user IDs in growing order until February 2024, so an
//! ID below 7·10⁹ points at a year. Since then IDs are random inside a block
//! that stays open for a few months, so an ID says only when its block was
//! open.

/// When an account was registered, to the year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationEstimate {
    Year(i32),
    /// The ID block was open across a year boundary.
    Years {
        from: i32,
        to: i32,
    },
    /// The ID block was still open, or not yet known, when the table was
    /// last updated.
    YearOrLater(i32),
}

use RegistrationEstimate::{Year, YearOrLater, Years};

/// The first user ID of each range and the estimate for the range; `None`
/// for IDs Telegram never handed out. Sorted by ID.
///
/// Derived from the account-age tables of LyAdminBot
/// (<https://github.com/LyoSU/LyAdminBot>, `packages/adapters/src/account-age.ts`
/// as of 2026-08-05), copyright (c) 2019 Yuri Ly, MIT licence. Below 7·10⁹
/// only the IDs where its curve crosses 1 January (UTC) are kept; above,
/// its blocks are merged by the years they were open.
///
/// Updated by hand. When a new block opens: turn the `YearOrLater` entry
/// into the years its block was open and add the new block as `YearOrLater`
/// of its opening year.
const RANGES: [(i64, Option<RegistrationEstimate>); 19] = [
    (1, Some(Year(2013))),
    (7_800_236, Some(Year(2014))),
    (82_410_182, Some(Year(2015))),
    (158_938_935, Some(Year(2016))),
    (329_099_693, Some(Year(2017))),
    (502_512_806, Some(Year(2018))),
    (819_513_438, Some(Year(2019))),
    (1_056_734_363, Some(Year(2020))),
    (1_580_004_326, Some(Year(2021))),
    // The switch from 32-bit to 64-bit IDs left this range empty.
    (1 << 31, None),
    (5_000_000_000, Some(Year(2021))),
    (5_081_730_117, Some(Year(2022))),
    (5_977_933_779, Some(Year(2023))),
    (6_992_079_548, Some(Year(2024))),
    (
        7_600_000_000,
        Some(Years {
            from: 2024,
            to: 2025,
        }),
    ),
    (8_200_000_000, Some(Year(2025))),
    (
        8_500_000_000,
        Some(Years {
            from: 2025,
            to: 2026,
        }),
    ),
    (8_600_000_000, Some(Year(2026))),
    // Still open when the table was last updated.
    (8_800_000_000, Some(YearOrLater(2026))),
];

pub fn registration_estimate(user_id: i64) -> Option<RegistrationEstimate> {
    let range = RANGES.partition_point(|&(first_id, _)| first_id <= user_id);
    RANGES.get(range.checked_sub(1)?)?.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_ids_point_at_the_year_of_a_nearby_known_registration() {
        // IDs with a registration time known to the source table.
        for (user_id, year) in [
            (1_000_000, 2013),
            (11_538_514, 2014),
            (101_260_938, 2015),
            (171_295_414, 2016),
            (337_808_429, 2017),
            (539_850_643, 2018),
            (824_044_288, 2019),
            (1_057_704_545, 2020),
            (1_585_583_199, 2021),
            (2_138_472_342, 2021),
            (5_000_533_025, 2021),
            (5_107_165_824, 2022),
            (5_982_648_124, 2023),
            (6_984_062_645, 2023),
        ] {
            assert_eq!(
                registration_estimate(user_id),
                Some(Year(year)),
                "{user_id}"
            );
        }
    }

    #[test]
    fn ids_below_the_first_known_registration_are_from_the_first_year() {
        assert_eq!(registration_estimate(1), Some(Year(2013)));
        assert_eq!(registration_estimate(777_000), Some(Year(2013)));
    }

    #[test]
    fn ids_that_were_never_handed_out_have_no_estimate() {
        for user_id in [i64::MIN, -1, 0, 1 << 31, 4_999_999_999] {
            assert_eq!(registration_estimate(user_id), None, "{user_id}");
        }
        assert_eq!(registration_estimate((1 << 31) - 1), Some(Year(2021)));
        assert_eq!(registration_estimate(5_000_000_000), Some(Year(2021)));
    }

    #[test]
    fn block_ids_get_the_years_their_block_was_open() {
        for (user_id, estimate) in [
            (7_000_000_000, Year(2024)),
            (7_599_999_999, Year(2024)),
            (
                7_600_000_000,
                Years {
                    from: 2024,
                    to: 2025,
                },
            ),
            (
                8_199_999_999,
                Years {
                    from: 2024,
                    to: 2025,
                },
            ),
            (8_200_000_000, Year(2025)),
            (8_499_999_999, Year(2025)),
            (
                8_500_000_000,
                Years {
                    from: 2025,
                    to: 2026,
                },
            ),
            (
                8_599_999_999,
                Years {
                    from: 2025,
                    to: 2026,
                },
            ),
            (8_600_000_000, Year(2026)),
            (8_799_999_999, Year(2026)),
        ] {
            assert_eq!(registration_estimate(user_id), Some(estimate), "{user_id}");
        }
    }

    #[test]
    fn ids_in_the_open_block_and_above_it_are_from_its_opening_year_or_later() {
        for user_id in [8_800_000_000, 9_000_000_000, i64::MAX] {
            assert_eq!(
                registration_estimate(user_id),
                Some(YearOrLater(2026)),
                "{user_id}"
            );
        }
    }
}
