use kube::{Client, api::{Api, ApiResource, DynamicObject, ListParams, LogParams, DeleteParams}, core::GroupVersionKind};
use k8s_openapi::api::core::v1::{Pod, PodSpec, Container, Event};

#[derive(Debug, Clone)]
pub struct CephInfo {
    pub name: String,
    pub health: String
}

#[derive(Debug, Clone)]
pub struct CephPoolInfo {
    pub name: String,
    pub status: String,
    pub size: i64,
}

#[derive(Debug, Clone)]
pub struct PodInfo {
    pub name: String,
    pub status: String,
    pub ready: String,
    pub node: String,
    pub restarts: i32,
    pub containers: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn get_ceph_health(client: &Client) -> Result<Vec<CephInfo>> {
    let gvk = GroupVersionKind::gvk("ceph.rook.io", "v1", "CephCluster");    
    let api_resource = ApiResource::from_gvk_with_plural(&gvk, "cephclusters");
    let ceph_clusters: Api<DynamicObject> = Api::namespaced_with(client.clone(), "rook-ceph", &api_resource);
    let list = ceph_clusters.list(&ListParams::default()).await?;

    let mut res = Vec::new();
    for cluster in list.items {
        if let Some(status) = cluster.data.get("status").and_then(|s| s.get("ceph")) {
            let name = cluster.metadata.name.as_deref().unwrap_or("Unknown").to_string();
            let health = status.get("health").and_then(|h| h.as_str()).unwrap_or("Unknown").to_string();
            res.push(CephInfo {name, health});
        }
    }
    
    Ok(res)
}

pub async fn get_ceph_pools(client: &Client) -> Result<Vec<CephPoolInfo>> {
    let gvk = GroupVersionKind::gvk("ceph.rook.io", "v1", "CephBlockPool");
    let api_resource = ApiResource::from_gvk_with_plural(&gvk, "cephblockpools");
    let pools: Api<DynamicObject> = Api::namespaced_with(client.clone(), "rook-ceph", &api_resource);
    let list = pools.list(&ListParams::default()).await?;

    let mut res = Vec::new();
    for pool in list.items {
        let name = pool.metadata.name.as_deref().unwrap_or("Unknown").to_string();
        let status = pool.data.get("status").and_then(|s| s.get("phase")).and_then(|p| p.as_str()).unwrap_or("Unknown").to_string();
        let size = pool.data.get("spec").and_then(|s| s.get("replicated")).and_then(|r| r.get("size")).and_then(|sz| sz.as_i64()).unwrap_or(0);
        res.push(CephPoolInfo { name, status, size });
    }
    Ok(res)
}

pub async fn get_pods(client: &Client) -> Result<Vec<PodInfo>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), "rook-ceph");
    let list = pods.list(&ListParams::default()).await?;

    let mut res = Vec::new();
    for pod in list.items {
        let name = pod.metadata.name.as_deref().unwrap_or("Unknown").to_string();
        let status = pod.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("Unknown").to_string();
        let node = pod.spec.as_ref().and_then(|s| s.node_name.as_deref()).unwrap_or("Unknown").to_string();
        
        let container_statuses = pod.status.as_ref().and_then(|s| s.container_statuses.as_ref());
        let restarts = container_statuses.map(|cs| cs.iter().map(|c| c.restart_count).sum()).unwrap_or(0);
        
        let total_containers = pod.spec.as_ref().map(|s| s.containers.len()).unwrap_or(0);
        let ready_containers = container_statuses.map(|cs| cs.iter().filter(|c| c.ready).count()).unwrap_or(0);
        let ready = format!("{}/{}", ready_containers, total_containers);

        let mut containers = Vec::new();
        if let Some(spec) = &pod.spec {
            for c in &spec.containers {
                containers.push(c.name.clone());
            }
        }
        
        res.push(PodInfo { name, status, ready, node, restarts, containers });
    }
    Ok(res)
}

pub async fn restart_pod(client: &Client, pod_name: &str) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), "rook-ceph");
    pods.delete(pod_name, &DeleteParams::default()).await?;
    Ok(())
}

pub async fn delete_pod(client: &Client, pod_name: &str) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), "rook-ceph");
    pods.delete(pod_name, &DeleteParams::default()).await?;
    Ok(())
}

pub async fn fetch_pod_logs(client: &Client, pod_name: &str, container_name: Option<String>) -> Result<Vec<String>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), "rook-ceph");
    
    // If container_name is not specified, we attempt to get the pod to find the first container name
    let container = if let Some(c) = container_name {
        Some(c)
    } else {
        let pod = pods.get(pod_name).await?;
        pod.spec.and_then(|s| s.containers.first().map(|c| c.name.clone()))
    };

    let logs_params = LogParams {
        tail_lines: Some(100),
        container,
        ..LogParams::default()
    };
    
    let logs_str = pods.logs(pod_name, &logs_params).await?;
    let mut res = Vec::new();
    for line in logs_str.lines() {
        res.push(line.to_string());
    }
    Ok(res)
}

pub async fn describe_pod(client: &Client, pod_name: &str) -> Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), "rook-ceph");
    let pod = pods.get(pod_name).await?;
    
    let pod_yaml = serde_yaml::to_string(&pod)?;
    
    let events_api: Api<Event> = Api::namespaced(client.clone(), "rook-ceph");
    let lp = ListParams::default().fields(&format!("involvedObject.name={}", pod_name));
    let events = events_api.list(&lp).await?;
    
    let mut describe_text = format!("--- POD YAML ---\n{}\n\n--- EVENTS ---\n", pod_yaml);
    
    if events.items.is_empty() {
        describe_text.push_str("No events found.\n");
    } else {
        describe_text.push_str(&format!("{:<20} {:<10} {:<15} {:<20}\n", "LAST SEEN", "TYPE", "REASON", "MESSAGE"));
        for e in events.items {
            let last_seen = e.last_timestamp.as_ref().map(|t| format!("{:?}", t.0)).unwrap_or_else(|| "Unknown".to_string());
            let etype = e.type_.as_deref().unwrap_or("Normal");
            let reason = e.reason.as_deref().unwrap_or("Unknown");
            let message = e.message.as_deref().unwrap_or("");
            describe_text.push_str(&format!("{:<20} {:<10} {:<15} {:<20}\n", 
                &last_seen[..std::cmp::min(20, last_seen.len())], etype, reason, message));
        }
    }
    
    Ok(describe_text)
}

pub async fn delete_pool(client: &Client, pool_name: &str) -> Result<()> {
    let gvk = GroupVersionKind::gvk("ceph.rook.io", "v1", "CephBlockPool");
    let api_resource = ApiResource::from_gvk_with_plural(&gvk, "cephblockpools");
    let pools: Api<DynamicObject> = Api::namespaced_with(client.clone(), "rook-ceph", &api_resource);
    pools.delete(pool_name, &DeleteParams::default()).await?;
    Ok(())
}