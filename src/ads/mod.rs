use crate::config::{AdsConfig, AdsterraConfig, HouseAdSlot};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AdResponse {
    pub slots: Vec<AdSlotResult>,
    pub refresh_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smartlink_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdSlotResult {
    pub slot: String,
    pub provider: &'static str,
    pub html: String,
    pub cpm: f64,
}

pub struct AdEngine {
    config: AdsConfig,
}

impl AdEngine {
    pub fn new(config: AdsConfig) -> Self {
        Self { config }
    }

    pub fn collect(&self) -> AdResponse {
        let slots = self.collect_slots();
        let smartlink_url = self.config.adsterra.iter()
            .find(|a| a.slot == "terra")
            .and_then(|a| a.redirect_url.clone());
        AdResponse {
            refresh_secs: self.config.refresh_secs,
            slots,
            smartlink_url,
        }
    }

    fn collect_slots(&self) -> Vec<AdSlotResult> {
        let mut slot_map: std::collections::HashMap<String, Vec<AdSlotResult>> =
            std::collections::HashMap::new();

        for ad in &self.config.house {
            let result = AdSlotResult {
                slot: ad.slot.clone(),
                provider: "house",
                html: self.house_html(ad),
                cpm: ad.cpm.unwrap_or(0.50),
            };
            slot_map.entry(ad.slot.clone()).or_default().push(result);
        }

        for ad in &self.config.adsterra {
            let result = AdSlotResult {
                slot: ad.slot.clone(),
                provider: "adsterra",
                html: self.adsterra_html(ad),
                cpm: ad.cpm.unwrap_or(1.00),
            };
            slot_map.entry(ad.slot.clone()).or_default().push(result);
        }

        let mut out = Vec::new();
        for (_slot_name, candidates) in slot_map {
            let best = candidates
                .into_iter()
                .max_by(|a, b| a.cpm.partial_cmp(&b.cpm).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(ad) = best {
                out.push(ad);
            }
        }

        out
    }

    fn adsterra_html(&self, ad: &AdsterraConfig) -> String {
        if let Some(src) = &ad.script_src {
            if let Some(cid) = &ad.container_id {
                format!(
                    r#"<div id="{cid}"></div><script async src="{src}"></script>"#,
                    cid = cid, src = src,
                )
            } else {
                format!(
                    r#"<script type="text/javascript" src="{src}"></script>"#,
                    src = src,
                )
            }
        } else if let Some(url) = &ad.redirect_url {
            format!(
                r#"<a href="{url}" target="_blank" rel="sponsored noopener" class="ad-link">Patrocinado &rsaquo;</a>"#,
                url = url,
            )
        } else {
            String::new()
        }
    }

    fn house_html(&self, house: &HouseAdSlot) -> String {
        let alt = house
            .alt
            .as_deref()
            .unwrap_or("Anúncio")
            .replace('"', "&quot;");
        format!(
            r#"<a href="{dest}" target="_blank" rel="sponsored noopener"><img src="{img}" alt="{alt}" style="width:100%;height:auto;border-radius:6px;display:block" /></a>"#,
            dest = house.dest_url,
            img = house.image_url,
            alt = alt,
        )
    }
}
