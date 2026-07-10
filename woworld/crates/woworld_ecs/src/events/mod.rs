//! 事件系统 — EventChannel<E> 双缓冲事件队列
//!
//! 帧内时序: begin_frame → send (write_buf) → mid_phase_flush → read (read_buf)
//! 设计 005 §四 规定。
//!
//! 修正: 设计原版的 begin_frame 使用 swap+clear 逻辑有误——本轮实现使用
//! 简化版（read_buf.clear()），不丢失跨帧事件。

/// 双缓冲事件通道——泛型 E。
///
/// - `write_buf`: 当前帧系统写入事件
/// - `read_buf`: 上一帧刷入、当前帧可读
#[derive(Debug, Clone)]
pub struct EventChannel<E> {
    write_buf: Vec<E>,
    read_buf: Vec<E>,
}

impl<E> Default for EventChannel<E> {
    fn default() -> Self {
        Self {
            write_buf: Vec::new(),
            read_buf: Vec::new(),
        }
    }
}

impl<E> EventChannel<E> {
    /// 新建空通道。
    pub fn new() -> Self {
        Self::default()
    }

    /// 帧开始——清空 read_buf（上一帧事件已消费完毕）。
    pub fn begin_frame(&mut self) {
        self.read_buf.clear();
    }

    /// 发送事件到 write_buf。
    ///
    /// 调用时机：Phase 1 中段——ActionController。
    pub fn send(&mut self, event: E) {
        self.write_buf.push(event);
    }

    /// 批量发送事件。
    pub fn send_all(&mut self, events: impl IntoIterator<Item = E>) {
        self.write_buf.extend(events);
    }

    /// 中段刷新——write_buf → read_buf（中段 System 此后可见）。
    ///
    /// 调用时机：Block A2.5 结束（ActionController 完成后）。
    pub fn mid_phase_flush(&mut self) {
        self.read_buf.append(&mut self.write_buf);
    }

    /// 读取已刷新事件（read_buf）。
    ///
    /// 调用时机：Phase 1 中段/晚段——GOAP/Memory/Animation。
    pub fn read(&self) -> &[E] {
        &self.read_buf
    }

    /// 取走所有事件（消费后清空 read_buf）。
    pub fn drain(&mut self) -> Vec<E> {
        std::mem::take(&mut self.read_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_channel_new_is_empty() {
        let ch: EventChannel<i32> = EventChannel::new();
        assert!(ch.read().is_empty());
    }

    #[test]
    fn test_event_send_and_flush() {
        let mut ch = EventChannel::new();
        ch.send(42);
        assert!(ch.read().is_empty()); // 尚未 flush
        ch.mid_phase_flush();
        assert_eq!(ch.read(), &[42]);
    }

    #[test]
    fn test_event_begin_frame_clears_read() {
        let mut ch = EventChannel::new();
        ch.send(1);
        ch.mid_phase_flush();
        assert_eq!(ch.read().len(), 1);
        ch.begin_frame();
        assert!(ch.read().is_empty());
    }

    #[test]
    fn test_event_send_all() {
        let mut ch = EventChannel::new();
        ch.send_all(vec![1, 2, 3]);
        ch.mid_phase_flush();
        assert_eq!(ch.read(), &[1, 2, 3]);
    }

    #[test]
    fn test_event_drain_consumes() {
        let mut ch = EventChannel::new();
        ch.send(10);
        ch.mid_phase_flush();
        let drained = ch.drain();
        assert_eq!(drained, vec![10]);
        assert!(ch.read().is_empty());
    }
}
