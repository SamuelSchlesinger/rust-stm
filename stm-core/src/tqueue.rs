// Copyright 2015-2016 rust-stm Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::sync::Arc;

use crate::{StmError, TVar, Transaction};
use archery::shared_pointer::kind::ArcK;
use rpds::List;

#[derive(Debug)]
pub struct TQueue<A: Sync + Send + 'static> {
    read: TVar<rpds::List<Arc<A>, ArcK>>,
    write: TVar<rpds::List<Arc<A>, ArcK>>,
}

impl<A: Sync + Send + 'static> Clone for TQueue<A> {
    fn clone(&self) -> Self {
        TQueue {
            read: self.read.clone(),
            write: self.write.clone(),
        }
    }
}

impl<A: Send + Sync + 'static> TQueue<A> {
    pub fn new() -> Self {
        TQueue {
            read: TVar::new(rpds::List::new_sync()),
            write: TVar::new(rpds::List::new_sync()),
        }
    }

    pub fn write(&self, mut txn: &mut Transaction, item: Arc<A>) -> Result<(), StmError> {
        let list = self.write.read(&mut txn)?;
        self.write.write(&mut txn, list.push_front(item))?;

        Ok(())
    }

    pub fn read(&self, mut txn: &mut Transaction) -> Result<Arc<A>, StmError> {
        let xs = self.read.read(&mut txn)?;
        if xs.is_empty() {
            let ys = self.write.read(&mut txn)?;
            if ys.is_empty() {
                Err(StmError::Retry)
            } else {
                let zs = ys.reverse();
                self.write.write(&mut txn, List::new_sync())?;
                self.read.write(&mut txn, zs.drop_first().unwrap())?;
                Ok(zs.first().unwrap().clone())
            }
        } else {
            self.read.write(&mut txn, xs.drop_first().unwrap())?;
            Ok(xs.first().unwrap().clone())
        }
    }
}

#[test]
fn test_tqueue() {
    let tqueue: TQueue<i32> = TQueue::new();
    assert_eq!(
        *crate::atomically(|txn| {
            tqueue.write(txn, Arc::new(1))?;
            tqueue.read(txn)
        }),
        1
    );
    crate::atomically(|mut txn| {
        tqueue.write(&mut txn, Arc::new(1))?;
        Ok(())
    });
    assert_eq!(*crate::atomically(|mut txn| { tqueue.read(&mut txn) }), 1);
}
