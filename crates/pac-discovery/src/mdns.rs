use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use pac_core::types::NodeInfo;
use crate::registry::NodeRegistry;

/// 局域網 Discovery 服務廣播器與接收器 (基於 UDP 廣播模擬 mDNS / DNS-SD 機制)
pub struct LanDiscoveryService {
    registry: NodeRegistry,
    broadcast_port: u16,
}

impl LanDiscoveryService {
    pub fn new(registry: NodeRegistry, broadcast_port: u16) -> Self {
        Self {
            registry,
            broadcast_port,
        }
    }

    /// 啟動被動監聽來自局域網邊緣節點的註冊與心跳廣播
    pub async fn start_listener(self: Arc<Self>) {
        let bind_addr: SocketAddr = format!("0.0.0.0:{}", self.broadcast_port)
            .parse()
            .expect("Invalid broadcast bind addr");

        let socket = match UdpSocket::bind(bind_addr).await {
            Ok(s) => {
                info!("PAC-Discovery: UDP mDNS 廣播監聽器已綁定於 {}", bind_addr);
                s
            }
            Err(e) => {
                warn!("PAC-Discovery: 無法綁定廣播端口 {} ({})，切換為僅內存註冊模式", bind_addr, e);
                return;
            }
        };

        let mut buf = vec![0u8; 4096];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, peer)) => {
                    if let Ok(node_info) = serde_json::from_slice::<NodeInfo>(&buf[..len]) {
                        info!("PAC-Discovery: 收到來自 {} 的節點自動註冊: {} ({})", peer, node_info.name, node_info.id);
                        self.registry.register_or_update(node_info).await;
                    }
                }
                Err(e) => {
                    error!("PAC-Discovery 監聽錯誤: {}", e);
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// 主動向局域網廣播本機節點資訊
    pub async fn broadcast_presence(node: &NodeInfo, broadcast_port: u16) -> anyhow::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        let target_addr: SocketAddr = format!("255.255.255.255:{}", broadcast_port).parse()?;
        let data = serde_json::to_vec(node)?;
        socket.send_to(&data, target_addr).await?;
        Ok(())
    }
}

