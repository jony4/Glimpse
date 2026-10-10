use super::Workspace;
use gpui_kit::*;

pub(super) struct HistorySwipe {
    started: std::time::Instant,
    x: f32,
    y: f32,
    offsets: Vec<Pixels>,
}
impl Workspace {
    pub(super) fn history_swipe(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ScrollDelta::Pixels(delta) = event.delta else {
            return;
        };
        if event.modifiers != Modifiers::default() {
            self.history_swipe = None;
            return;
        }
        if event.touch_phase == TouchPhase::Started {
            self.history_swipe = self
                .active_reader()
                .map_or(Some(Vec::new()), |reader| reader.horizontal_offsets(cx))
                .map(|offsets| HistorySwipe {
                    started: std::time::Instant::now(),
                    x: 0.,
                    y: 0.,
                    offsets,
                });
        }
        if event.touch_phase == TouchPhase::Cancelled {
            self.history_swipe = None;
            return;
        }
        let Some(swipe) = &mut self.history_swipe else {
            return;
        };
        swipe.x += f32::from(delta.x);
        swipe.y += f32::from(delta.y).abs();
        if event.touch_phase != TouchPhase::Ended {
            return;
        }
        let swipe = self.history_swipe.take().unwrap();
        let offsets = self
            .active_reader()
            .map_or(Some(Vec::new()), |reader| reader.horizontal_offsets(cx));
        let stationary = offsets.is_some_and(|offsets| {
            offsets.len() == swipe.offsets.len()
                && offsets
                    .iter()
                    .zip(&swipe.offsets)
                    .all(|(a, b)| f32::from(*a - *b).abs() < 1.)
        });
        if stationary
            && swipe.started.elapsed().as_millis() <= 800
            && swipe.x.abs() >= 80.
            && swipe.x.abs() > swipe.y * 2.
        {
            self.navigate(swipe.x < 0., window, cx);
            cx.stop_propagation();
        }
    }
}
