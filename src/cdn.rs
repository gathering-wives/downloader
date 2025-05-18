use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
#[allow(dead_code)]
pub struct Resource {
    pub dest: String,
    pub md5: String,
    pub sampleHash: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResourcesResponse {
    pub resource: Vec<Resource>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CDNEntry {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct DefaultIndex {
    pub cdnList: Vec<CDNEntry>,
    pub resources: String,
    pub resourcesBasePath: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexResponse {
    pub default: DefaultIndex,
}
