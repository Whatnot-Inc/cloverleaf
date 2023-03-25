use simple_grad::*;

use crate::feature_store::FeatureStore;
use crate::embeddings::EmbeddingStore;
use crate::algos::ep::model::attention_mean;

pub trait EmbeddingBuilder {
    fn construct( &self, features: &[usize], out: &mut [f32]) -> ();
}

pub struct AvgAggregator<'a> {
    embs: &'a EmbeddingStore
}

impl <'a> AvgAggregator<'a> {
    pub fn new(embs: &'a EmbeddingStore) -> Self {
        AvgAggregator { embs }
    }

    #[inline]
    fn isum(out: &mut [f32], x: &[f32]) {
        out.iter_mut().zip(x.iter()).for_each(|(outi, xi)| {
            *outi += xi;
        });
    }
}
impl <'a> EmbeddingBuilder for AvgAggregator<'a> {
    fn construct(
        &self, 
        features: &[usize],
        out: &mut [f32]
    ) {
        out.fill(0f32);
        for feat_id in features {
            let e = self.embs.get_embedding(*feat_id); 
            AvgAggregator::isum(out, e);
        }

        let len = features.len() as f32;
        out.iter_mut().for_each(|outi| *outi /= len);
    }
}

pub struct UnigramProbability {
    p_w: Vec<f32>
}

impl UnigramProbability {
    pub fn new(features: &FeatureStore) -> Self{
        let mut counts = vec![0usize; features.num_features()];
        let mut total = 0usize;
        for feats in features.iter() {
            for feat_id in feats.iter() {
                counts[*feat_id] += 1;
                total += 1;
            }
        }

        let p_w = counts.into_iter()
            .map(|c| c as f32 / total as f32)
            .collect();

        UnigramProbability { p_w }
    }

    pub fn from_vec(p_w: Vec<f32>) -> Self {
        UnigramProbability { p_w }
    }

    pub fn iter(&self) -> impl Iterator<Item=&f32> {
        self.p_w.iter()
    }
}

pub struct WeightedAggregator<'a> {
    embs: &'a EmbeddingStore,
    up: &'a UnigramProbability,
    alpha: f32
}

impl <'a> WeightedAggregator<'a> {
    pub fn new(embs: &'a EmbeddingStore, up: &'a UnigramProbability, alpha: f32) -> Self {
        WeightedAggregator { embs, up, alpha }
    }

    #[inline]
    fn imulsum(out: &mut [f32], x: &[f32], scalar: f32) {
        out.iter_mut().zip(x.iter()).for_each(|(outi, xi)| {
            *outi += scalar * xi;
        });
    }
}

impl <'a> EmbeddingBuilder for WeightedAggregator<'a> {
    fn construct(
        &self, 
        features: &[usize],
        out: &mut [f32]
    ) {
        out.fill(0f32);
        let mut weight = 0f32;
        for feat_id in features.iter() {
            let p_wi = self.up.p_w[*feat_id];
            let w = self.alpha / (self.alpha + p_wi);
            weight += w;
            let e = self.embs.get_embedding(*feat_id); 
            WeightedAggregator::imulsum(out, e, w);
        }

        out.iter_mut().for_each(|outi| *outi /= weight);
    }
}

pub struct AttentionAggregator<'a> {
    embs: &'a EmbeddingStore,
    dims: usize,
    window: Option<usize>
}

impl <'a> AttentionAggregator<'a> {
    pub fn new(embs: &'a EmbeddingStore, dims: usize, window: Option<usize>) -> Self {
        AttentionAggregator { embs, dims, window }
    }

}

impl <'a> EmbeddingBuilder for AttentionAggregator<'a> {
    fn construct(
        &self, 
        features: &[usize],
        out: &mut [f32]
    ) {
        out.fill(0f32);
        let it = features.iter().map(|feat_id| {
            let e = self.embs.get_embedding(*feat_id); 
            (Constant::new(e.to_vec()), 1usize)
        }).collect::<Vec<_>>();

        let v = attention_mean(it.iter(), self.dims, self.window);
        v.value().iter().zip(out.iter_mut()).for_each(|(vi, outi)| {
            *outi = *vi;
        });
    }
}

