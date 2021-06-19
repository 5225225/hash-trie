use crate::{flag::*, traits::*, node::*, result::*, HashTrieError};
use alloc::{fmt::Debug};
use core::hash::Hash;

#[derive(Debug)]
pub(crate) struct HashTrie <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> {
    root: MNode<H, F, K, V, M>,
}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> HashTrie<H, F, K, V, M> where <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            root: MNode::default()
        }
    }

    #[must_use]
    fn singleton(mnode: MNode<H, F, K, V, M>) -> Self {
        Self {
            root: mnode
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.root.size()
    }

    pub(crate) fn find<'a, L: Key + HashLike<K>>(&'a self, key: &L) -> Result<(&'a K, &'a V), HashTrieError> where K: PartialEq<L>, M: HasherBv<H, L>, <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {
        match self.root.find(key, Some(Flag::new(M::default().hash(key)))) {
            FindResult::NotFound => Err(HashTrieError::NotFound),
            FindResult::Found(key, value) => Ok((key, value))
        }
    }

    pub(crate) fn insert<'a, L: Key + Into<K> + Hash + HashLike<K>, W: Into<V>>(&'a self, key: L, value: W, replace: bool) -> Result<(Self, *const K, *const V, Option<(&'a K, &'a V)>), (&'a K, &'a V)>
    where
        K: HashLike<L>,
        K: PartialEq<L>,
        M: HasherBv<H, L>
    {
        let flag = Flag::from(M::default().hash(&key));
        match self.root.insert(key, value, Some(flag), replace) {
            InsertResult::Found(key, value) => Err((key, value)),
            InsertResult::InsertedC(cnode, key, value, prev) => Ok((Self::singleton(MNode::C(cnode)), key, value, prev)),
            InsertResult::InsertedL(lnode, key, value, prev) => Ok((Self::singleton(MNode::L(lnode)), key, value, prev)),
            InsertResult::InsertedS(snode, key, value, prev) => Ok((Self::singleton(MNode::S(snode)), key, value, prev)),
        }
    }

    pub(crate) fn remove<'a, L: Key + HashLike<K>>(&'a self, key: &L) -> Result<(Self, &'a K, &'a V), HashTrieError> where K: PartialEq<L>, M: HasherBv<H, L> {
        match self.root.remove(key, Some(Flag::from(M::default().hash(key)))) {
            RemoveResult::NotFound => Err(HashTrieError::NotFound),
            RemoveResult::RemovedC(cnode, key, value) => Ok((Self::singleton(MNode::C(cnode)), key, value)),
            RemoveResult::RemovedL(lnode, key, value) => Ok((Self::singleton(MNode::L(lnode)), key, value)),
            RemoveResult::RemovedS(snode, key, value) => Ok((Self::singleton(MNode::S(snode)), key, value)),
            RemoveResult::RemovedZ(key, value) => Ok((Self::default(), key, value))
        }
    }
    
    pub(crate) fn visit<Op: Clone>(&self, op: Op) where Op: Fn(&K, &V) {
        self.root.visit(op);
    }

    pub(crate) fn transform<ReduceT, ReduceOp, Op>
        (&self, reduce_op: ReduceOp, op: Op) -> (Self, ReduceT)
        where
        Self: Sized,
        ReduceT: Default,
        ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
        Op: Fn(&K, &V) -> MapTransformResult<V, ReduceT> + Clone,
    {
        match self.root.transform(reduce_op, op) {
            MNodeTransformResult::Unchanged(reduced) => (self.clone(), reduced),
            MNodeTransformResult::C(cnode, reduced) => (Self::singleton(MNode::C(cnode)), reduced),
            MNodeTransformResult::L(lnode, reduced) => (Self::singleton(MNode::L(lnode)), reduced),
            MNodeTransformResult::S(snode, reduced) => (Self::singleton(MNode::S(snode)), reduced),
            MNodeTransformResult::Removed(reduced) => (Self::default(), reduced),
        }
    }

    pub(crate) fn transmute<S: Key, X: Value, ReduceT, ReduceOp, Op>
        (&self, reduce_op: ReduceOp, op: Op) -> (HashTrie<H, F, S, X, M>, ReduceT)
        where
        Self: Sized,
        ReduceT: Default,
        ReduceOp: Fn(&ReduceT, &ReduceT) -> ReduceT + Clone,
        Op: Fn(&K, &V) -> MapTransmuteResult<S, X, ReduceT> + Clone,
        K: HashLike<S>,
        K: PartialEq<S>,
        M: HasherBv<H, S>,
    {
        match self.root.transmute(reduce_op, op) {
            MNodeTransmuteResult::C(cnode, reduced) => (HashTrie::singleton(MNode::C(cnode)), reduced),
            MNodeTransmuteResult::L(lnode, reduced) => (HashTrie::singleton(MNode::L(lnode)), reduced),
            MNodeTransmuteResult::S(snode, reduced) => (HashTrie::singleton(MNode::S(snode)), reduced),
            MNodeTransmuteResult::Removed(reduced) => (HashTrie::default(), reduced),
        }
    }

    pub(crate) fn joint_transmute<L: Key, W: Value, S: Key, X: Value, ReduceT, ReduceOp, BothOp, LeftOp, RightOp>
        (&self, right: &HashTrie<H, F, L, W, M>, reduce_op: ReduceOp, both_op: BothOp, left_op: LeftOp, right_op: RightOp) -> (HashTrie<H, F, S, X, M>, ReduceT)
        where
        Self: Sized,
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
    {
        match self.root.joint_transmute(&right.root, reduce_op, both_op, left_op, right_op, 0) {
            MNodeTransmuteResult::C(cnode, reduced) => (HashTrie::singleton(MNode::C(cnode)), reduced),
            MNodeTransmuteResult::L(lnode, reduced) => (HashTrie::singleton(MNode::L(lnode)), reduced),
            MNodeTransmuteResult::S(snode, reduced) => (HashTrie::singleton(MNode::S(snode)), reduced),
            MNodeTransmuteResult::Removed(reduced) => (HashTrie::default(), reduced),
        }
    }

}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> Clone for HashTrie<H, F, K, V, M> where <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {
    fn clone(&self) -> Self {
        Self::singleton(self.root.clone())
    }
}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> Default for HashTrie<H, F, K, V, M> where <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {
    fn default() -> Self {
        Self::new()
    }
}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> Eq for HashTrie<H, F, K, V, M> where <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {}

impl <H: Hashword, F: Flagword<H>, K: Key, V: Value, M: HasherBv<H, K>> PartialEq for HashTrie<H, F, K, V, M> where <F as core::convert::TryFrom<<H as core::ops::BitAnd>::Output>>::Error: core::fmt::Debug {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}
