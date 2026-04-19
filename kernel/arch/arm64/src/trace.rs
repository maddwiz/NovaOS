#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceKind {
    Boot,
    Memory,
    Scheduler,
    Driver,
    Panic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub kind: TraceKind,
    pub code: u32,
    pub value: u64,
}

impl TraceEvent {
    pub const fn new(kind: TraceKind, code: u32, value: u64) -> Self {
        Self { kind, code, value }
    }
}

pub struct TraceBuffer<'a> {
    slots: &'a mut [TraceEvent],
    cursor: usize,
}

impl<'a> TraceBuffer<'a> {
    pub fn new(slots: &'a mut [TraceEvent]) -> Self {
        Self { slots, cursor: 0 }
    }

    pub fn record(&mut self, event: TraceEvent) {
        if self.slots.is_empty() {
            return;
        }

        let index = self.cursor % self.slots.len();
        self.slots[index] = event;
        self.cursor = self.cursor.wrapping_add(1);
    }
}
