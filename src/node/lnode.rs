use crate::{flag::*, result::*, traits::*};
use super::{cnode::{self}, mnode::*, snode::{self, *}};
use alloc::{fmt::Debug, sync::*, vec::Vec};

#[derive(Clone, Debug)]
pub(crate) enum LNodeNext<K: Key, V: Value> {
    L(Arc<LNode<K, V>>),
    S(Arc<SNode<K, V>>),
}

impl <K: Key, V: Value> LNodeNext<K, V> {
    fn key(&self) -> &K {
        match self {
            LNodeNext::L(lnode) => lnode.key(),
            LNodeNext::S(snode) => snode.key(),
        }
    }
}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> From<LNodeNext<K, V>> for MNode<H, F, K, V, M> {
    fn from(other: LNodeNext<K, V>) -> Self {
        match other {
            LNodeNext::L(lnode) => MNode::L(lnode),
            LNodeNext::S(snode) => MNode::S(snode),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LNode<K: Key, V: Value> {
    key: K,
    value: V,
    next: LNodeNext<K, V>,
    size: usize,
}

impl<K: Key, V: Value> LNode<K, V> {
    #[must_use]
    pub(super) fn key(&self) -> &K {
        &self.key
    }

    #[must_use]
    pub(super) fn value(&self) -> &V {
        &self.value
    }

    #[must_use]
    pub(super) fn new(key: K, value: V, next: LNodeNext<K, V>) -> Arc<Self> {
        let size = 1 + match &next {
            LNodeNext::L(lnode) => lnode.size,
            LNodeNext::S(_snode) => 1,
        };
        Arc::new(Self {
            key,
            value,
            next,
            size,
        })
    }

    #[must_use]
    pub(super) fn size(&self) -> usize {
        self.size
    }

    pub(super) fn visit<Op>(&self, op: Op) where Op: Fn(&K, &V) {
        op(&self.key, &self.value);
        match &self.next {
            LNodeNext::L(lnode) => lnode.visit(op),
            LNodeNext::S(snode) => snode.visit(op),
        }
    }

    pub(super) fn find<'a, L: Key>(&'a self, key: &L) -> FindResult<'a, K, V> where K: PartialEq<L> {
        if self.key == *key {
            FindResult::Found(self.key(), self.value())
        }
        else {
            match &self.next {
                LNodeNext::L(lnode) => lnode.find(key),
                LNodeNext::S(snode) => snode.find(key),
            }
        }
    }
    
}

pub(super) fn insert<'a, H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key + Into<K>, W: Into<V>, M: HasherBv<H, K>>(this: &'a Arc<LNode<K, V>>, key: L, value: W, key_flag: Option<Flag<H, F>>, replace: bool) -> LNodeInsertResult<'a, H, F, K, V, M>
where
    K: HashLike<L>,
    K: PartialEq<L>,
    M: HasherBv<H, L>,
    <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug
{
    match this.find(&key) {
        FindResult::Found(k, v) => if replace {
            match remove_from_lnode(this, &key) {
                LNodeRemoveResult::RemovedL(lnode, k, v) => {
                    let lnode = LNode::new(key.into(), value.into(), LNodeNext::L(lnode));
                    let key: *const K = lnode.key();
                    let value: *const V = lnode.value();
                    LNodeInsertResult::InsertedL(lnode, key, value, Some((k, v)))
                },
                LNodeRemoveResult::RemovedS(snode, k, v) => {
                    let lnode = LNode::new(key.into(), value.into(), LNodeNext::S(snode));
                    let key: *const K = lnode.key();
                    let value: *const V = lnode.value();
                    LNodeInsertResult::InsertedL(lnode, key, value, Some((k, v)))
                },
                LNodeRemoveResult::NotFound => panic!(),
            }
        }
        else {
            LNodeInsertResult::Found(k, v)
        },
        FindResult::NotFound => lift_to_cnode_and_insert(LNodeNext::L(this.clone()), key, value, key_flag.unwrap())
    }
}

pub(super) fn remove<'a, H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key, M: HasherBv<H, K>>(this: &'a Arc<LNode<K, V>>, key: &L) -> RemoveResult<'a, H, F, K, V, M> where K: PartialEq<L> {
    match remove_from_lnode(this, key) {
        LNodeRemoveResult::NotFound => RemoveResult::NotFound,
        LNodeRemoveResult::RemovedL(lnode, key, value) => RemoveResult::RemovedL(lnode, key, value),
        LNodeRemoveResult::RemovedS(snode, key, value) => RemoveResult::RemovedS(snode, key, value),
    }
}

fn remove_from_lnode<'a, K: Key, V: Value, L: Key>(this: &'a Arc<LNode<K, V>>, key: &L) -> LNodeRemoveResult<'a, K, V> where K: PartialEq<L> {
    if this.key == *key {
        match &this.next {
            LNodeNext::L(lnode) => LNodeRemoveResult::RemovedL(lnode.clone(), this.key(), this.value()),
            LNodeNext::S(snode) => LNodeRemoveResult::RemovedS(snode.clone(), this.key(), this.value()),
        }
    }
    else {
        match &this.next {
            LNodeNext::L(lnode) => match remove_from_lnode(lnode, key) {
                LNodeRemoveResult::NotFound => LNodeRemoveResult::NotFound,
                LNodeRemoveResult::RemovedL(lnode, key, value) => LNodeRemoveResult::RemovedL(LNode::new(this.key.clone(), this.value.clone(), LNodeNext::L(lnode)), key, value),
                LNodeRemoveResult::RemovedS(snode, key, value) => LNodeRemoveResult::RemovedL(LNode::new(this.key.clone(), this.value.clone(), LNodeNext::S(snode)), key, value),
            },
            LNodeNext::S(snode) => match snode::remove(snode, key) {
                SNodeRemoveResult::NotFound => LNodeRemoveResult::NotFound,
                SNodeRemoveResult::RemovedZ(key, value) => LNodeRemoveResult::RemovedS(SNode::new(this.key.clone(), this.value.clone()), key, value),
            }
        }
    }
}

pub(super) fn transform<K: Key, V: Value, ReduceT, ReduceOp, Op>(this: &Arc<LNode<K, V>>, reduce_op: ReduceOp, op: Op) -> LNodeTransformResult<K, V, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    Op: Fn(&K, &V) -> MapTransformResult<V, ReduceT> + Clone,
{
    let next = match &this.next {
        LNodeNext::L(lnode) => transform(lnode, reduce_op.clone(), op.clone()),
        LNodeNext::S(snode) => snode::transform(snode, op.clone()).into(),
    };

    transform_result(this, op(&this.key, &this.value), next, reduce_op)
}

pub(super) fn transmute<K: Key, V: Value, S: Key, X: Value, ReduceT, ReduceOp, Op>(this: &Arc<LNode<K, V>>, reduce_op: ReduceOp, op: Op) -> LNodeTransmuteResult<S, X, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    Op: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<S>,
    K: PartialEq<S>,
{
    let next = match &this.next {
        LNodeNext::L(lnode) => transmute(lnode, reduce_op.clone(), op.clone()),
        LNodeNext::S(snode) => snode::transmute(snode, op.clone()).into(),
    };

    transmute_result(op(&this.key, &this.value), next, reduce_op)
}

pub(crate) fn joint_transmute<H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key, S: Key, W: Value, X: Value, M: HasherBv<H, K>, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>(this: &Arc<LNode<K, V>>, right: &MNode<H, F, L, W, M>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp, depth: usize) -> MNodeTransmuteResult<H, F, S, X, M, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    BothOp: Fn(&K, &V, &L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    LeftOp: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    RightOp: Fn(&L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<L>,
    K: PartialEq<L>,
    K: HashLike<S>,
    K: PartialEq<S>,
    L: HashLike<K>,
    L: PartialEq<K>,
    L: HashLike<S>,
    L: PartialEq<S>,
    M: HasherBv<H, L>,
    M: HasherBv<H, S>,
    <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug
{
    match right {
        MNode::C(cnode) => cnode::joint_transmute_lnode(cnode, this, |a,b| reduce_op(b, a), |k,v,l,w| both_op(l, w, k, v), right_op, left_op, depth),
        MNode::L(lnode) => joint_transmute_lnode(this, lnode, reduce_op, both_op, left_op, right_op, depth),
        MNode::S(snode) => joint_transmute_snode(this, snode, reduce_op, both_op, left_op, right_op, depth),
    }
}

pub(crate) fn joint_transmute_lnode<H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key, W: Value, S: Key, X: Value, M: HasherBv<H, K>, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>(this: &Arc<LNode<K, V>>, right: &Arc<LNode<L, W>>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp, depth: usize) -> MNodeTransmuteResult<H, F, S, X, M, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    BothOp: Fn(&K, &V, &L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    LeftOp: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    RightOp: Fn(&L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<L>,
    K: PartialEq<L>,
    K: HashLike<S>,
    K: PartialEq<S>,
    L: HashLike<K>,
    L: PartialEq<K>,
    L: HashLike<S>,
    L: PartialEq<S>,
    M: HasherBv<H, L>,
    M: HasherBv<H, S>,
    <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug
{
    let this_hash = M::default().hash(this.key());
    let right_hash = M::default().hash(right.key());

    if this_hash == right_hash {
        let mut rights = vec!((&right.key, &right.value));
        let mut r = right;
        loop {
            match &r.next {
                LNodeNext::L(lnode) => {
                    rights.push((&lnode.key, &lnode.value));
                    r = right;
                },
                LNodeNext::S(snode) => {
                    rights.push((snode.key(), snode.value()));
                    break;
                }
            }
        }
    
        let mut next = joint_transmute_lnode_impl(this, &mut rights, reduce_op.clone(), both_op, left_op, right_op.clone());
    
        for (rk, rv) in rights {
            next = match right_op(rk, rv) {
                MapTransmuteResult::Transmuted(rk, rv, rr) => match next {
                    LNodeTransmuteResult::L(lnode, rn) => LNodeTransmuteResult::L(LNode::new(rk, rv, LNodeNext::L(lnode)), reduce_op(&rr, &rn)),
                    LNodeTransmuteResult::S(snode, rn) => LNodeTransmuteResult::L(LNode::new(rk, rv, LNodeNext::S(snode)), reduce_op(&rr, &rn)),
                    LNodeTransmuteResult::Removed(rn) => LNodeTransmuteResult::S(SNode::new(rk, rv), reduce_op(&rr, &rn)),
                },
                MapTransmuteResult::Removed(rr) => match next {
                    LNodeTransmuteResult::L(lnode, rn) => LNodeTransmuteResult::L(lnode, reduce_op(&rr, &rn)),
                    LNodeTransmuteResult::S(snode, rn) => LNodeTransmuteResult::S(snode, reduce_op(&rr, &rn)),
                    LNodeTransmuteResult::Removed(rn) => LNodeTransmuteResult::Removed(reduce_op(&rr, &rn)),
                },
            };
        }
    
        next.into()
    }
    else {
        let this = transmute(this, reduce_op.clone(), left_op);
        let right = transmute(right, reduce_op.clone(), right_op);

        let this_flag = Flag::new_at_depth(this_hash, depth).unwrap();
        let right_flag = Flag::new_at_depth(right_hash, depth).unwrap();

        match this {
            LNodeTransmuteResult::L(llnode, lr) => match right {
                LNodeTransmuteResult::L(rlnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(llnode.into(), this_flag, rlnode.into(), right_flag), reduce_op(&lr, &rr)),
                LNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(llnode.into(), this_flag, rsnode.into(), right_flag), reduce_op(&lr, &rr)),
                LNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::L(llnode, reduce_op(&lr, &rr)),
            },
            LNodeTransmuteResult::S(lsnode, lr) => match right {
                LNodeTransmuteResult::L(rlnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(lsnode.into(), this_flag, rlnode.into(), right_flag), reduce_op(&lr, &rr)),
                LNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(lsnode.into(), this_flag, rsnode.into(), right_flag), reduce_op(&lr, &rr)),
                LNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::S(lsnode, reduce_op(&lr, &rr)),
            },
            LNodeTransmuteResult::Removed(lr) => match right {
                LNodeTransmuteResult::L(rlnode, rr) => MNodeTransmuteResult::L(rlnode, reduce_op(&lr, &rr)),
                LNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::S(rsnode, reduce_op(&lr, &rr)),
                LNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::Removed(reduce_op(&lr, &rr)),
            },
        }
    }
}

fn joint_transmute_lnode_impl<K: Key, V: Value, L: Key, W: Value, S: Key, X: Value, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>(this: &Arc<LNode<K, V>>, rights: &mut Vec<(&L, &W)>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp) -> LNodeTransmuteResult<S, X, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    BothOp: Fn(&K, &V, &L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    LeftOp: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    RightOp: Fn(&L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<L>,
    K: PartialEq<L>,
    K: HashLike<S>,
    K: PartialEq<S>,
    L: HashLike<K>,
    L: PartialEq<K>,
    L: HashLike<S>,
    L: PartialEq<S>,
{
    let next = match &this.next {
        LNodeNext::L(lnode) => joint_transmute_lnode_impl(lnode, rights, reduce_op.clone(), both_op.clone(), left_op.clone(), right_op.clone()),
        LNodeNext::S(snode) => {
            match rights.iter().position(|key_value| *snode.key() == *key_value.0).map(|index| rights.swap_remove(index)) {
                Some((right_key, right_value)) => snode::joint_transmute_values(snode.key(), snode.value(), right_key, right_value, reduce_op.clone(), both_op.clone(), left_op.clone(), right_op.clone()),
                None => snode::transmute(snode, left_op.clone()).into(),
            }
        },
    };

    let result = match rights.iter().position(|key_value| this.key == *key_value.0).map(|index| rights.swap_remove(index)) {
        Some((right_key, right_value)) => both_op(&this.key, &this.value, right_key, right_value),
        None => left_op(&this.key, &this.value),
    };

    transmute_result(result, next, reduce_op)
}

pub(crate) fn joint_transmute_snode<H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key, W: Value, S: Key, X: Value, M: HasherBv<H, K>, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>(this: &Arc<LNode<K, V>>, right: &Arc<SNode<L, W>>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp, depth: usize) -> MNodeTransmuteResult<H, F, S, X, M, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    BothOp: Fn(&K, &V, &L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    LeftOp: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    RightOp: Fn(&L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<L>,
    K: PartialEq<L>,
    K: HashLike<S>,
    K: PartialEq<S>,
    L: HashLike<K>,
    L: PartialEq<K>,
    L: HashLike<S>,
    L: PartialEq<S>,
    M: HasherBv<H, L>,
    M: HasherBv<H, S>,
    <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug
{
    let this_hash = M::default().hash(this.key());
    let right_hash = M::default().hash(right.key());

    if this_hash == right_hash {
        joint_transmute_snode_impl(this, right, reduce_op, both_op, left_op, right_op).into()
    }
    else {
        let this = transmute(this, reduce_op.clone(), left_op);
        let right = snode::transmute(right, right_op);

        let this_flag = Flag::new_at_depth(this_hash, depth).unwrap();
        let right_flag = Flag::new_at_depth(right_hash, depth).unwrap();

        match this {
            LNodeTransmuteResult::L(llnode, lr) => match right {
                SNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(llnode.into(), this_flag, rsnode.into(), right_flag), reduce_op(&lr, &rr)),
                SNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::L(llnode, reduce_op(&lr, &rr)),
            },
            LNodeTransmuteResult::S(lsnode, lr) => match right {
                SNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::C(cnode::lift_to_cnode_and_insert(lsnode.into(), this_flag, rsnode.into(), right_flag), reduce_op(&lr, &rr)),
                SNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::S(lsnode, reduce_op(&lr, &rr)),
            },
            LNodeTransmuteResult::Removed(lr) => match right {
                SNodeTransmuteResult::S(rsnode, rr) => MNodeTransmuteResult::S(rsnode, reduce_op(&lr, &rr)),
                SNodeTransmuteResult::Removed(rr) => MNodeTransmuteResult::Removed(reduce_op(&lr, &rr)),
            },
        }
    }
}

pub(crate) fn joint_transmute_snode_impl<K: Key, V: Value, L: Key, S: Key, W: Value, X: Value, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>(this: &Arc<LNode<K, V>>, right: &Arc<SNode<L, W>>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp) -> LNodeTransmuteResult<S, X, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
    BothOp: Fn(&K, &V, &L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    LeftOp: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    RightOp: Fn(&L, &W) -> MapTransmuteResult<S, X, ReduceT> + Clone,
    K: HashLike<L>,
    K: PartialEq<L>,
    K: HashLike<S>,
    K: PartialEq<S>,
    L: HashLike<K>,
    L: PartialEq<K>,
    L: HashLike<S>,
    L: PartialEq<S>,
{
    if this.key == *right.key() {
        let next = match &this.next {
            LNodeNext::L(lnode) => transmute(lnode, reduce_op.clone(), left_op),
            LNodeNext::S(snode) => snode::transmute(snode, left_op).into(),
        };
        let this = both_op(this.key(), this.value(), right.key(), right.value());

        transmute_result(this, next, reduce_op)
    }
    else {
        let next = match &this.next {
            LNodeNext::L(lnode) => joint_transmute_snode_impl(lnode, right, reduce_op.clone(), both_op, left_op.clone(), right_op),
            LNodeNext::S(snode) => snode::joint_transmute_values(snode.key(), snode.value(), right.key(), right.value(), reduce_op.clone(), both_op, left_op.clone(), right_op),
        };
        let this = left_op(this.key(), this.value());

        transmute_result(this, next, reduce_op)
    }
}

pub(super) fn transform_result<K: Key, V: Value, ReduceT, ReduceOp>(this: &Arc<LNode<K, V>>, result: MapTransformResult<V, ReduceT>, next: LNodeTransformResult<K, V, ReduceT>, reduce_op: ReduceOp) -> LNodeTransformResult<K, V, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone
{
    match result {
        MapTransformResult::Unchanged(lr) => match next {
            LNodeTransformResult::Unchanged(rr) => LNodeTransformResult::Unchanged(reduce_op(&lr, &rr)),
            LNodeTransformResult::L(lnode, rr) => LNodeTransformResult::L(LNode::new(this.key.clone(), this.value.clone(), LNodeNext::L(lnode)), reduce_op(&lr, &rr)),
            LNodeTransformResult::S(snode, rr) => LNodeTransformResult::L(LNode::new(this.key.clone(), this.value.clone(), LNodeNext::S(snode)), reduce_op(&lr, &rr)),
            LNodeTransformResult::Removed(rr) => LNodeTransformResult::S(SNode::new(this.key.clone(), this.value.clone()), reduce_op(&lr, &rr)),
        },
        MapTransformResult::Transformed(lv, lr) => match next {
            LNodeTransformResult::Unchanged(rr) => LNodeTransformResult::L(LNode::new(this.key.clone(), lv, this.next.clone()), reduce_op(&lr, &rr)),
            LNodeTransformResult::L(lnode, rr) => LNodeTransformResult::L(LNode::new(this.key.clone(), lv, LNodeNext::L(lnode)), reduce_op(&lr, &rr)),
            LNodeTransformResult::S(snode, rr) => LNodeTransformResult::L(LNode::new(this.key.clone(), lv, LNodeNext::S(snode)), reduce_op(&lr, &rr)),
            LNodeTransformResult::Removed(rr) => LNodeTransformResult::S(SNode::new(this.key.clone(), lv), reduce_op(&lr, &rr)),
        },
        MapTransformResult::Removed(lr) => match next {
            LNodeTransformResult::Unchanged(rr) => match &this.next {
                LNodeNext::L(lnode) => LNodeTransformResult::L(lnode.clone(), reduce_op(&lr, &rr)),
                LNodeNext::S(snode) => LNodeTransformResult::S(snode.clone(), reduce_op(&lr, &rr)),
            },
            LNodeTransformResult::L(lnode, rr) => LNodeTransformResult::L(lnode, reduce_op(&lr, &rr)),
            LNodeTransformResult::S(snode, rr) => LNodeTransformResult::S(snode, reduce_op(&lr, &rr)),
            LNodeTransformResult::Removed(rr) => LNodeTransformResult::Removed(reduce_op(&lr, &rr)),
        },
    }
}

pub(super) fn transmute_result<S: Key, X: Value, ReduceT, ReduceOp>(result: MapTransmuteResult<S, X, ReduceT>, next: LNodeTransmuteResult<S, X, ReduceT>, reduce_op: ReduceOp) -> LNodeTransmuteResult<S, X, ReduceT>
where
    ReduceT: Default,
    ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone
{
    match result {
        MapTransmuteResult::Transmuted(lk, lv, lr) => match next {
            LNodeTransmuteResult::L(lnode, rr) => LNodeTransmuteResult::L(LNode::new(lk, lv, LNodeNext::L(lnode)), reduce_op(&lr, &rr)),
            LNodeTransmuteResult::S(snode, rr) => LNodeTransmuteResult::L(LNode::new(lk, lv, LNodeNext::S(snode)), reduce_op(&lr, &rr)),
            LNodeTransmuteResult::Removed(rr) => LNodeTransmuteResult::S(SNode::new(lk, lv), reduce_op(&lr, &rr)),
        },
        MapTransmuteResult::Removed(lr) => match next {
            LNodeTransmuteResult::L(lnode, rr) => LNodeTransmuteResult::L(lnode, reduce_op(&lr, &rr)),
            LNodeTransmuteResult::S(snode, rr) => LNodeTransmuteResult::S(snode, reduce_op(&lr, &rr)),
            LNodeTransmuteResult::Removed(rr) => LNodeTransmuteResult::Removed(reduce_op(&lr, &rr)),
        },
    }
}

#[must_use]
pub(super) fn lift_to_cnode_and_insert<'a, H: Hashword, F: Flagword<H>, K: Key, V: Value, L: Key + Into<K>, W: Into<V>, M: HasherBv<H, K>>(this: LNodeNext<K, V>, key: L, value: W, key_flag: Flag<H, F>) -> LNodeInsertResult<'a, H, F, K, V, M>
where
    K: HashLike<L>,
    K: PartialEq<L>,
    M: HasherBv<H, L>,
    <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug
{
    let this_hash = M::default().hash(this.key());
    if this_hash == key_flag.hash_value() {
        let lnode = LNode::new(key.into(), value.into(), this);
        let key: *const K = lnode.key();
        let value: *const V = lnode.value();
        LNodeInsertResult::InsertedL(lnode, key, value, None)
    }
    else {
        let this_flag = Flag::new_at_depth(this_hash, key_flag.depth()).unwrap();

        let snode = SNode::new(key.into(), value.into());
        let key: *const K = snode.key();
        let value: *const V = snode.value();
        LNodeInsertResult::InsertedC(cnode::lift_to_cnode_and_insert(this.into(), this_flag, snode.into(), key_flag), key, value, None)
    }
}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> From<Arc<LNode<K, V>>> for MNode<H, F, K, V, M> {
    fn from(other: Arc<LNode<K, V>>) -> Self {
        MNode::L(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    macro_rules! lnode {
        ( $keyvalue1:expr, $keyvalue2:expr ) => {
            {
                LNode::new($keyvalue1.0, $keyvalue1.1, LNodeNext::S(SNode::new($keyvalue2.0, $keyvalue2.1)))
            }
        };
        ( $keyvalue:expr, $($rest:expr),+ ) => {
            {
                LNode::new($keyvalue.0, $keyvalue.1, LNodeNext::L(lnode!($($rest),*)))
            }
        };
    }

    #[test]
    fn lnode_insert_3() {
        let node = lnode!((3, ()), (2, ()), (1, ()));
        assert_eq!(node.size, 3);
        assert_found_eq!(node.find(&1), (1, ()));
        assert_found_eq!(node.find(&2), (2, ()));
        assert_found_eq!(node.find(&3), (3, ()));
        assert_found_none!(node.find(&4));
    }

}
