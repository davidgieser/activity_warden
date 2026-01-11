use plotters::prelude::*;
use plotters_cairo::CairoBackend;

use relm4::gtk;
use relm4::prelude::*;
use relm4::gtk::prelude::*;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use shared::dbus::Host;
use zbus::blocking::Connection as BlockingConnection;
use plotters::style::text_anchor::{Pos, HPos, VPos};
use plotters::style::RGBColor;

use shared::types::daemon::DurationMap;
use crate::{Duration, DurationId};

/// The number of names that are displayed within the histogram.
const HIST_SIZE: usize = 10;

#[derive(Default, Clone, Debug)]
struct HistState {
    names: Vec<String>,
    durations: Vec<Duration>,
}

#[derive(Debug)]
pub struct DataPage {
    _dbus_conn: BlockingConnection,
    histogram: Rc<RefCell<HistState>>,
    timer_durations: Rc<RefCell<DurationMap>>,

    /// Used to iterate, in order, over the display names for a 
    /// given host with the largest durations.
    sorted_durations: HashMap<Host, BTreeSet<DurationId>>,
    active_host: Host,
}

#[derive(Debug)]
pub struct DataInit {
    pub dbus_conn: BlockingConnection,
    pub timer_durations: Rc<RefCell<DurationMap>>,
}

#[derive(Debug)]
pub enum DataInput {
    /// Take the old duration Id to clear stale state and the 
    /// new duration to update the graphs.
    DurationUpdate(DurationId, Duration),
    DurationsLoaded,
}

#[derive(Debug)]
pub enum DataOut { }

fn palette_from_style() -> (RGBColor, RGBColor, RGBColor, RGBColor) {
    let dark = adw::StyleManager::default().is_dark();
    if dark {
        (RGBColor(30,30,34), RGBColor(220,220,225), RGBColor(80,80,90), RGBColor(102,153,255))
    } else {
        (WHITE, BLACK, RGBColor(200,200,210), RGBColor(45,95,200))
    }
}


#[relm4::component(pub)]
impl Component for DataPage {
    type Init = DataInit;
    type Input = DataInput;
    type Output = DataOut;
    type CommandOutput = ();

    view! {
        #[name = "drawing_area"]
        gtk::DrawingArea {
            set_content_width: 640,
            set_content_height: 360,
        }
    }
    
    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DataPage {
            _dbus_conn: init.dbus_conn,
            histogram: Rc::new(RefCell::new(HistState {names: Vec::new(), durations: Vec::new()})),
            timer_durations: init.timer_durations,
            sorted_durations: HashMap::new(),
            active_host: Host::FirefoxWatcher,
        };
        let widgets = view_output!();

        // Set the function to actually draw the histogram.
        widgets.drawing_area.set_draw_func({
            let histogram = model.histogram.clone();
            
            move |_, cr, w, h| {
                let (bg, fg, grid, _accent) = palette_from_style();

                let palette: [&RGBColor; 6] = [
                    &RGBColor(180, 70, 70),   // muted red
                    &RGBColor(70, 120, 170),  // steel blue
                    &RGBColor(90, 150, 90),   // soft green
                    &RGBColor(150, 90, 150),  // dusty purple
                    &RGBColor(80, 160, 160),  // muted teal
                    &RGBColor(200, 170, 80),  // warm ochre
                ];

                let backend = CairoBackend::new(cr, (w as u32, h as u32)).unwrap();
                let root = backend.into_drawing_area();
                let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
                    root.fill(&bg)?;

                    // Determine the data to display from the internal state.
                    let hist_state = (*histogram).borrow();
                    let mut bins = hist_state.names.clone();
                    let mut durations = hist_state.durations.iter()
                        .map(|a| (a + 59) / 60)
                        .collect::<Vec<Duration>>();

                    if bins.is_empty() && durations.is_empty() {
                        durations = vec![0];
                        bins = vec!["".to_string()];
                    }

                    const LABEL_OFFSET: usize = 2;
                    const FONT_NAME: &str = "Inter";

                    let x_label_style = TextStyle::from((FONT_NAME, 12).into_font())
                        .color(&fg)
                        .pos(Pos::new(HPos::Left, VPos::Center));

                    // Determine the longest label that might be drawn at label end.
                    let longest_label = bins.iter()
                        .map(|s| if s.is_empty() { "firefox_tools" } else { s.as_str() })
                        .max_by_key(|s| s.len())
                        .unwrap_or("")
                        .to_string();

                    // Measure the label size in pixels.
                    let (label_w_px, _label_h_px) = root.estimate_text_size(&longest_label, &x_label_style)?;
                    let max_duration = durations.iter().copied().max().unwrap_or(0);
                    let mut x_range: usize = max_duration + LABEL_OFFSET + 1;

                    // Draw a temporary graph to capture the pixel width and optimal label padding.
                    {
                        let tmp = ChartBuilder::on(&root)
                            .caption("Top 10 Sites Used Today", (FONT_NAME, 24).into_font().color(&fg))
                            .margin(12)
                            .x_label_area_size(40)
                            .y_label_area_size(40)
                            .build_cartesian_2d(0..x_range, (0..bins.len()).into_segmented())?;

                        let plot = tmp.plotting_area();

                        // Get the plotting area pixel width
                        let (px, _py) = plot.get_pixel_range();
                        let plot_w_px = (px.end - px.start).abs() as f64;

                        // Compute required padding for the label in x units.
                        let px_per_unit = (plot_w_px / (x_range.max(1) as f64)).max(0.1);
                        let raw_pad_units = (label_w_px as f64 / px_per_unit).ceil() as usize;
                        x_range += raw_pad_units;
                    }

                    // Build the final, displayed chart.
                    let y_range = (0..bins.len()).into_segmented();
                    let mut chart = ChartBuilder::on(&root)
                        .caption("Top 10 Sites Used Today", (FONT_NAME, 24).into_font().color(&fg))
                        .margin(12)
                        .x_label_area_size(40)
                        .y_label_area_size(40)
                        .build_cartesian_2d(0..x_range, y_range)?;

                    let font = (FONT_NAME, 12).into_font().color(&fg);
                    chart.configure_mesh()
                        .disable_x_mesh()
                        .label_style(font.clone())
                        .axis_style(&fg)
                        .light_line_style(&grid)
                        .y_labels(bins.len())
                        .y_label_formatter(&|_sv| { 
                            // Hide the y-axis labels.
                            "".to_string()
                        })
                        .x_desc("Duration (min)")
                        .draw()?;
                    
                    // Draw the histogram bars.
                    chart.draw_series((0..bins.len()).map(|i| {
                        let color = palette[i % palette.len()];
                        Rectangle::new(
                            [(0, SegmentValue::Exact(i)), (durations[i], SegmentValue::Exact(i+1))],
                            color.filled(),
                        )
                    }))?;

                    let plot = chart.plotting_area();

                    // Write the histogram bar text labels.
                    for i in 0..bins.len() {
                        let x = SegmentValue::CenterOf(i);
                        let y = durations[i] + LABEL_OFFSET;

                        let label_text = if bins[i] == "" {
                            "firefox_tools".to_string()
                        } else {
                            bins[i].clone()
                        };

                        let elem = Text::new(
                            label_text,
                            (y, x),
                            x_label_style.clone(),
                        );
                        plot.draw(&elem).unwrap();
                    }

                    let _ = root.present();
                    Ok(())
                })();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self, 
        msg: DataInput, 
        _sender: ComponentSender<Self>, 
        root: &Self::Root
    ) {
        match msg {
            DataInput::DurationUpdate(mut dur_id, new_dur) => {
                // Update the ordered set of durations to ensure accurate, in-order iteration
                // over display names with the largest durations.
                let sorted_host_set = self.sorted_durations.entry(dur_id.host.clone()).or_default();
                sorted_host_set.remove(&dur_id);
                dur_id.duration = new_dur as usize;
                sorted_host_set.insert(dur_id);

                let mut cur_hist = (*self.histogram).take();
                let (names, durations) = sorted_host_set.iter()
                    .rev()
                    .take(HIST_SIZE)
                    .map(|e| (e.display_name.clone(), e.duration))
                    .unzip();

                cur_hist.names = names;
                cur_hist.durations = durations;

                let _ = (*self.histogram).replace(cur_hist);
                root.queue_draw();
            },
            DataInput::DurationsLoaded => {
                // Process any duration updates and modify the internal state.
                for (host, dur_map) in (*self.timer_durations).borrow().iter() {
                    let mut host_set = BTreeSet::new();
                    dur_map.iter()
                        .for_each(|(display_name, duration)| {
                            host_set.insert(DurationId {
                                duration: duration.clone() as usize,
                                display_name: display_name.clone(),
                                host: host.clone(),
                            });
                        });
                    
                    self.sorted_durations.insert(host.clone(), host_set);
                }

                // Update the most visited sites.
                let sorted_host_set = self.sorted_durations.entry(self.active_host.clone()).or_default();
                let mut cur_hist = (*self.histogram).take();
                let (names, durations) = sorted_host_set.iter()
                    .rev()
                    .take(HIST_SIZE)
                    .map(|e| (e.display_name.clone(), e.duration))
                    .unzip();

                cur_hist.names = names;
                cur_hist.durations = durations;

                // Redraw the histogram on update.
                let _ = (*self.histogram).replace(cur_hist);
                root.queue_draw();
            }
        }
    }
}