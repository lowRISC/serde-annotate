use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Document;
use crate::{AnnotatedSerializer, Deserializer as AnnotatedDeserializer};

impl Serialize for Document {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        AnnotatedSerializer::try_specialize(
            serializer,
            |_| Ok(self.clone()),
            |_| {
                Err(serde::ser::Error::custom("Serializing document nodes is only supported with serde_annotate::AnnotatedSerializer"))
            },
        )
    }
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        AnnotatedDeserializer::try_specialize(
            deserializer,
            |deserializer| Ok(deserializer.doc.clone()),
            |_| {
                Err(serde::de::Error::custom(
                "Deserializing document nodes is only supported with serde_annotate::Deserializer",
                ))
            },
        )
    }
}
