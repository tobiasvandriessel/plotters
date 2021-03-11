use std::{cmp::max, marker::PhantomData};

use super::boxplot::{BoxplotOrient, BoxplotOrientH, BoxplotOrientV};
use crate::element::{Drawable, PointCollection};
use crate::style::{Color, ShapeStyle, BLACK};
use plotters_backend::{BackendCoord, DrawingBackend, DrawingErrorKind};

const DEFAULT_WIDTH: u32 = 10;

#[derive(Clone, Debug)]
pub struct BoxplotData {
    minimum: f64,
    lower_quartile: f64,
    median: f64,
    upper_quartile: f64,
    maximum: f64,
    outliers: Vec<f64>,
}

impl BoxplotData {
    // Extract a value representing the `pct` percentile of a
    // sorted `s`, using linear interpolation.
    fn percentile_of_sorted<T: Into<f64> + Copy>(s: &[T], pct: f64) -> f64 {
        assert!(!s.is_empty());
        if s.len() == 1 {
            return s[0].into();
        }
        assert!(0_f64 <= pct);
        let hundred = 100_f64;
        assert!(pct <= hundred);
        if (pct - hundred).abs() < std::f64::EPSILON {
            return s[s.len() - 1].into();
        }
        let length = (s.len() - 1) as f64;
        let rank = (pct / hundred) * length;
        let lower_rank = rank.floor();
        let d = rank - lower_rank;
        let n = lower_rank as usize;
        let lo = s[n].into();
        let hi = s[n + 1].into();
        lo + (hi - lo) * d
    }

    pub fn values(&self) -> [f32; 5] {
        [
            self.minimum as f32,
            self.lower_quartile as f32,
            self.median as f32,
            self.upper_quartile as f32,
            self.maximum as f32,
        ]
    }

    pub fn new<T: Into<f64> + Copy + PartialOrd>(values: &[T]) -> Self {
        let mut values = values.to_owned();
        values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

        let lower = BoxplotData::percentile_of_sorted(&values, 25_f64);
        let median = BoxplotData::percentile_of_sorted(&values, 50_f64);
        let upper = BoxplotData::percentile_of_sorted(&values, 75_f64);
        let iqr = upper - lower;
        let lower_fence = lower - 1.5 * iqr;
        let upper_fence = upper + 1.5 * iqr;

        let mut outliers = Vec::with_capacity(values.len() / 2);

        let mut minimum = None;
        let mut maximum = None;

        for v in values {
            if v.into() < lower_fence || v.into() > upper_fence {
                outliers.push(v.into());
            } else {
                if minimum.is_none() {
                    minimum = Some(v.into());
                }
                maximum = Some(v.into());
            }
        }

        assert!(minimum.is_some());
        assert!(maximum.is_some());

        Self {
            minimum: minimum.unwrap(),
            lower_quartile: lower,
            median,
            upper_quartile: upper,
            maximum: maximum.unwrap(),
            outliers
        }
    }
}
/// The boxplot element
pub struct BoxplotOutliers<K, O: BoxplotOrient<K, f32>> {
    style: ShapeStyle,
    width: u32,
    whisker_width: f64,
    offset: f64,
    key: K,
    values: [f32; 5],
    outliers: Vec<f32>,
    _p: PhantomData<O>,
}

impl<K: Clone> BoxplotOutliers<K, BoxplotOrientV<K, f32>> {
    /// Create a new vertical boxplot element.
    ///
    /// - `key`: The key (the X axis value)
    /// - `quartiles`: The quartiles values for the Y axis
    /// - **returns** The newly created boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_vertical("group", &quartiles);
    /// ```
    pub fn new_vertical(key: K, boxplot_data: &BoxplotData) -> Self {
        let outliers = boxplot_data.outliers.iter().map(|o| *o as f32).collect();
        Self {
            style: Into::<ShapeStyle>::into(&BLACK),
            width: DEFAULT_WIDTH,
            whisker_width: 1.0,
            offset: 0.0,
            key,
            values: boxplot_data.values(),
            outliers,
            _p: PhantomData,
        }
    }
}

impl<K: Clone> BoxplotOutliers<K, BoxplotOrientH<K, f32>> {
    /// Create a new horizontal boxplot element.
    ///
    /// - `key`: The key (the Y axis value)
    /// - `quartiles`: The quartiles values for the X axis
    /// - **returns** The newly created boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_horizontal("group", &quartiles);
    /// ```
    pub fn new_horizontal(key: K, boxplot_data: &BoxplotData) -> Self {
        let outliers = boxplot_data.outliers.iter().map(|o| *o as f32).collect();
        Self {
            style: Into::<ShapeStyle>::into(&BLACK),
            width: DEFAULT_WIDTH,
            whisker_width: 1.0,
            offset: 0.0,
            key,
            values: boxplot_data.values(),
            outliers,
            _p: PhantomData,
        }
    }
}

impl<K, O: BoxplotOrient<K, f32>> BoxplotOutliers<K, O> {
    /// Set the style of the boxplot.
    ///
    /// - `S`: The required style
    /// - **returns** The up-to-dated boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_horizontal("group", &quartiles).style(&BLUE);
    /// ```
    pub fn style<S: Into<ShapeStyle>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    /// Set the bar width.
    ///
    /// - `width`: The required width
    /// - **returns** The up-to-dated boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_horizontal("group", &quartiles).width(10);
    /// ```
    pub fn width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Set the width of the whiskers as a fraction of the bar width.
    ///
    /// - `whisker_width`: The required fraction
    /// - **returns** The up-to-dated boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_horizontal("group", &quartiles).whisker_width(0.5);
    /// ```
    pub fn whisker_width(mut self, whisker_width: f64) -> Self {
        self.whisker_width = whisker_width;
        self
    }

    /// Set the element offset on the key axis.
    ///
    /// - `offset`: The required offset (on the X axis for vertical, on the Y axis for horizontal)
    /// - **returns** The up-to-dated boxplot element
    ///
    /// ```rust
    /// use plotters::prelude::*;
    ///
    /// let quartiles = Quartiles::new(&[7, 15, 36, 39, 40, 41]);
    /// let plot = Boxplot::new_horizontal("group", &quartiles).offset(-5);
    /// ```
    pub fn offset<T: Into<f64> + Copy>(mut self, offset: T) -> Self {
        self.offset = offset.into();
        self
    }
}

impl<'a, K: Clone, O: BoxplotOrient<K, f32>> PointCollection<'a, (O::XType, O::YType)>
    for &'a BoxplotOutliers<K, O>
{
    type Point = (O::XType, O::YType);
    type IntoIter = Vec<Self::Point>;
    fn point_iter(self) -> Self::IntoIter {
        let mut points: Vec<Self::Point> = self.values
            .iter()
            .map(|v| O::make_coord(self.key.clone(), *v))
            .collect();
        for i in 0..self.outliers.len() {
            points.push(O::make_coord(self.key.clone(), self.outliers[i]));
        }
        points
    }
}

impl<K, DB: DrawingBackend, O: BoxplotOrient<K, f32>> Drawable<DB> for BoxplotOutliers<K, O> {
    fn draw<I: Iterator<Item = BackendCoord>>(
        &self,
        points: I,
        backend: &mut DB,
        _: (u32, u32),
    ) -> Result<(), DrawingErrorKind<DB::ErrorType>> {
        let points: Vec<_> = points.collect();
        if points.len() >= 5 {
            let width = f64::from(self.width);
            let moved = |coord| O::with_offset(coord, self.offset);
            let start_bar = |coord| O::with_offset(moved(coord), -width / 2.0);
            let end_bar = |coord| O::with_offset(moved(coord), width / 2.0);
            let start_whisker =
                |coord| O::with_offset(moved(coord), -width * self.whisker_width / 2.0);
            let end_whisker =
                |coord| O::with_offset(moved(coord), width * self.whisker_width / 2.0);

            // |---[   |  ]----|
            // ^________________
            backend.draw_line(
                start_whisker(points[0]),
                end_whisker(points[0]),
                &self.style,
            )?;

            // |---[   |  ]----|
            // _^^^_____________

            backend.draw_line(
                moved(points[0]),
                moved(points[1]),
                &self.style.color.to_backend_color(),
            )?;

            // |---[   |  ]----|
            // ____^______^_____
            let corner1 = start_bar(points[3]);
            let corner2 = end_bar(points[1]);
            let upper_left = (corner1.0.min(corner2.0), corner1.1.min(corner2.1));
            let bottom_right = (corner1.0.max(corner2.0), corner1.1.max(corner2.1));
            backend.draw_rect(upper_left, bottom_right, &self.style, false)?;

            // |---[   |  ]----|
            // ________^________
            backend.draw_line(start_bar(points[2]), end_bar(points[2]), &self.style)?;

            // |---[   |  ]----|
            // ____________^^^^_
            backend.draw_line(moved(points[3]), moved(points[4]), &self.style)?;

            // |---[   |  ]----|
            // ________________^
            backend.draw_line(
                start_whisker(points[4]),
                end_whisker(points[4]),
                &self.style,
            )?;

            for i in 5..points.len() {
                backend.draw_circle(moved(points[i]), (width / 2.0) as u32, &self.style, false)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_draw_v() {
        let root = MockedBackend::new(1024, 768).into_drawing_area();
        let chart = ChartBuilder::on(&root)
            .build_cartesian_2d(0..2, 0f32..100f32)
            .unwrap();

        let values = Quartiles::new(&[6]);
        assert!(chart
            .plotting_area()
            .draw(&Boxplot::new_vertical(1, &values))
            .is_ok());
    }

    #[test]
    fn test_draw_h() {
        let root = MockedBackend::new(1024, 768).into_drawing_area();
        let chart = ChartBuilder::on(&root)
            .build_cartesian_2d(0f32..100f32, 0..2)
            .unwrap();

        let values = Quartiles::new(&[6]);
        assert!(chart
            .plotting_area()
            .draw(&Boxplot::new_horizontal(1, &values))
            .is_ok());
    }
}