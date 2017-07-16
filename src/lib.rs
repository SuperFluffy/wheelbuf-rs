//! Multi-read no_std ring buffer
//!
//! The wheelbuffer crate offers a ringbuffer-like structure without a read
//! pointer, making multiple reads of a buffer possible. Instead of relying on
//! a fixed data structure as a backend, it is generic over a type `C` that
//! offers the slice interface, e.g. a vector or even a static array.
//!
//! The create performs no allocations itself and does not use the standard
//! library (`#![no_std]`).

#![no_std]

use core::cmp;
use core::borrow::Borrow;
use core::convert::AsRef;
use core::marker::PhantomData;
use core::fmt::Write;

/// A multi-read Ringbuffer.
///
/// The Write trait is implemented for `char` buffers, see below.
#[derive(Debug)]
pub struct WheelBuf<C, I>
    where C: AsMut<[I]> + AsRef<[I]>
{
    /// Backend store
    data: C,

    /// Insert position
    head: usize,

    /// Position of the first item to be pushed
    tail: usize,

    /// Number of items in the buffer
    len: usize,

    _pd: PhantomData<I>,
}
///
/// WheelBuf iterator
#[derive(Debug)]
pub struct WheelBufDrain<'a, I>
    where I: 'a,
{
    buffer: &'a [I],
    head: &'a mut usize,
    tail: &'a mut usize,
    len: &'a mut usize,
}

/// WheelBuf iterator
#[derive(Debug)]
pub struct WheelBufIter<'a, C, I>
    where C: AsMut<[I]> + AsRef<[I]>,
          I: 'a,
          C: 'a
{
    buffer: &'a WheelBuf<C, I>,
    cur: usize,
}

impl<C, I> WheelBuf<C, I>
    where C: AsMut<[I]> + AsRef<[I]>,
{
    /// Creates a new WheelBuf.
    ///
    /// `data` is a backing data structure that must be convertible into a
    /// slice. The `len()` of data determines the size of the buffer.
    #[inline]
    pub fn new(data: C) -> WheelBuf<C, I> {
        WheelBuf {
            data: data,
            head: 0,
            tail: 0,
            len: 0,
            _pd: PhantomData,
        }
    }

    // /// Add item to wheel buffer.
    // #[inline]
    // pub fn push(&mut self, item: I) {
    //     self.data.as_mut()[self.pos] = item;
    //     self.total += 1;
    //     self.pos = (self.pos + 1) % self.data.as_ref().len();
    // }

    /// Capacity of wheel buffer.
    ///
    /// Always equal to `len()` of underlying `data`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.as_ref().len()
    }

    /// Number of items in buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn head(&self) -> usize {
        self.head
    }

    pub fn tail(&self) -> usize {
        self.tail
    }

    /// Drains the buffer.
    #[inline]
    pub fn drain<'a>(&'a mut self) -> WheelBufDrain<'a, I> {
        WheelBufDrain {
            buffer: self.data.as_ref(),
            head: &mut self.head,
            tail: &mut self.tail,
            len: &mut self.len,
        }
    }

    /// Creates an iterator over buffer.
    #[inline]
    pub fn iter<'a>(&'a self) -> WheelBufIter<'a, C, I> {
        WheelBufIter {
            buffer: self,
            cur: 0,
        }
    }
}

impl<C, I> WheelBuf<C, I>
    where C: AsMut<[I]> + AsRef<[I]>,
          I: Clone,
{
    /// Push to the front of the wheel.
    #[inline]
    pub fn push<J: Borrow<I>>(&mut self, item: J) {
        self.data.as_mut()[self.head].clone_from(item.borrow());

        if self.tail == self.head && self.len > 0 {
            self.tail = (self.tail + 1) % self.capacity();
        }

        self.head = (self.head + 1) % self.capacity();

        if self.len < self.capacity() {
            self.len += 1;
        }
    }
}

impl<'a, I> Iterator for WheelBufDrain<'a, I>
    where I: 'a,
{
    type Item = &'a I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if *self.len > 0 {
            let elem = &self.buffer[*self.tail];

            // if *self.tail == *self.head {
            //     *self.tail = (*self.tail + 1) % self.buffer.len();
            //     *self.head = *self.tail;
            // } else {
            //     *self.tail = (*self.tail + 1) % self.buffer.len();
            // }
            *self.tail = (*self.tail + 1) % self.buffer.len();
            *self.len -= 1;

            Some(elem)
        } else {
            *self.head = 0;
            *self.tail = 0;
            None
        }
    }
}

impl<'a, C, I> Iterator for WheelBufIter<'a, C, I>
    where C: AsMut<[I]> + AsRef<[I]>,
          I: 'a,
          C: 'a
{
    type Item = &'a I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.buffer.len() {
            return None;
        }

        let cur = self.cur;
        self.cur += 1;
        Some(&self.buffer.data.as_ref()[(self.buffer.tail + cur) % self.buffer.capacity()])
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let max_idx = self.buffer.len;

        if n > 0 {
            self.cur += cmp::min(n, max_idx);
        }

        self.next()
    }
}

impl<C> Write for WheelBuf<C, char>
    where C: AsMut<[char]> + AsRef<[char]>
{
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        for c in s.chars() {
            self.push(c)
        }
        Ok(())
    }
}

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests {
    use core::fmt::Write;
    use std::string::String;
    use super::*;

    #[test]
    fn basics() {
        let mut buf = ['x'; 8];
        let mut wheel = WheelBuf::new(&mut buf);

        wheel.push('H');
        wheel.push('e');
        wheel.push('l');
        assert_eq!(wheel.len(), 3);
        assert_eq!(*wheel.iter().next().unwrap(), 'H');

        wheel.push('l');
        wheel.push('o');
        wheel.push(' ');
        wheel.push('W');
        wheel.push('o');
        wheel.push('r');
        wheel.push('l');
        wheel.push('d');
        assert_eq!(wheel.len(), 8);

        let s: String = wheel.iter().cloned().collect();
        assert_eq!(s.as_str(), "lo World");
    }

    #[test]
    fn clonable() {
        let mut buf = vec![vec!['x']; 8];
        let mut wheel = WheelBuf::new(&mut buf);

        wheel.push(vec!['H']);
        wheel.push(vec!['e']);
        wheel.push(vec!['l']);
        assert_eq!(wheel.len(), 3);
        assert_eq!(wheel.iter().next().unwrap()[0], 'H');

        wheel.push(vec!['l']);
        wheel.push(vec!['o']);
        wheel.push(vec![' ']);
        wheel.push(vec!['W']);
        wheel.push(vec!['o']);
        wheel.push(vec!['r']);
        wheel.push(vec!['l']);
        wheel.push(vec!['d']);
        assert_eq!(wheel.len(), 8);

        let mut iter = wheel.iter();
        assert_eq!(iter.next().unwrap()[0], 'l');
        assert_eq!(iter.next().unwrap()[0], 'o');
        assert_eq!(iter.next().unwrap()[0], ' ');
        assert_eq!(iter.next().unwrap()[0], 'W');
        assert_eq!(iter.next().unwrap()[0], 'o');
        assert_eq!(iter.next().unwrap()[0], 'r');
        assert_eq!(iter.next().unwrap()[0], 'l');
        assert_eq!(iter.next().unwrap()[0], 'd');
    }

    #[test]
    fn drain() {
        let mut buf = ['x'; 8];
        let mut wheel = WheelBuf::new(&mut buf);

        wheel.push('H');
        wheel.push('e');
        wheel.push('l');
        wheel.push('l');
        wheel.push('o');
        wheel.push(' ');
        wheel.push('W');
        wheel.push('o');
        wheel.push('r');
        wheel.push('l');
        wheel.push('d');
        assert_eq!(wheel.len(), 8);
        {
            let mut drain = wheel.drain();
            assert_eq!(*drain.next().unwrap(), 'l');
            assert_eq!(*drain.next().unwrap(), 'o');
        }

        assert_eq!(wheel.len(), 6);
        assert_eq!(wheel.head(), 3);
        assert_eq!(wheel.tail(), 5);
        let s: String = wheel.drain().cloned().collect();
        assert_eq!(s.as_str(), " World");

        assert_eq!(wheel.len(), 0);
        assert_eq!(wheel.head(), 0);
        assert_eq!(wheel.tail(), 0);
    }

    #[test]
    fn nth() {
        let mut buf = ['x'; 8];
        let mut wheel = WheelBuf::new(&mut buf);

        wheel.push('H');
        wheel.push('e');
        wheel.push('l');

        assert_eq!(*wheel.iter().nth(0).unwrap(), 'H');
        assert_eq!(*wheel.iter().nth(1).unwrap(), 'e');
        assert_eq!(*wheel.iter().nth(2).unwrap(), 'l');
        assert!(wheel.iter().nth(3).is_none());
    }

    #[test]
    fn write() {
        let mut buf = ['x'; 8];
        let mut wheel = WheelBuf::new(&mut buf);

        write!(wheel, "Hello, World! {}", 123).unwrap();
        let s: String = wheel.iter().cloned().collect();
        assert_eq!(s.as_str(), "rld! 123");
    }

    #[test]
    fn using_vec() {
        let mut buf = vec!['x', 'x', 'x', 'x', 'x', 'x', 'x', 'x'];
        let mut wheel = WheelBuf::new(&mut buf);

        wheel.push('H');
        wheel.push('e');
        wheel.push('l');
        assert_eq!(wheel.len(), 3);
        assert_eq!(*wheel.iter().next().unwrap(), 'H');

        wheel.push('l');
        wheel.push('o');
        wheel.push(' ');
        wheel.push('W');
        wheel.push('o');
        wheel.push('r');
        wheel.push('l');
        wheel.push('d');
        assert_eq!(wheel.len(), 8);

        let s: String = wheel.iter().cloned().collect();
        assert_eq!(s.as_str(), "lo World");
    }
}
