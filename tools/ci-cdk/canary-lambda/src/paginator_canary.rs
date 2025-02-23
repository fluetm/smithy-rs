/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

use crate::canary::Clients;

use crate::mk_canary;
use anyhow::bail;

use aws_sdk_ec2 as ec2;
use aws_sdk_ec2::model::InstanceType;

use crate::CanaryEnv;
use tokio_stream::StreamExt;

mk_canary!("ec2_paginator", |clients: &Clients, env: &CanaryEnv| {
    paginator_canary(clients.ec2.clone(), env.page_size)
});

pub async fn paginator_canary(client: ec2::Client, page_size: usize) -> anyhow::Result<()> {
    let mut history = client
        .describe_spot_price_history()
        .instance_types(InstanceType::M1Medium)
        .into_paginator()
        .page_size(page_size as i32)
        .send();

    let mut num_pages = 0;
    while let Some(page) = history.try_next().await? {
        let items_in_page = page.spot_price_history.unwrap_or_default().len();
        if items_in_page > page_size as usize {
            bail!(
                "failed to retrieve results of correct page size (expected {}, got {})",
                page_size,
                items_in_page
            )
        }
        num_pages += 1;
    }
    if dbg!(num_pages) < 2 {
        bail!(
            "expected 3+ pages containing ~60 results but got {} pages",
            num_pages
        )
    }

    // https://github.com/awslabs/aws-sdk-rust/issues/405
    let _ = client
        .describe_vpcs()
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::paginator_canary::paginator_canary;

    #[tokio::test]
    async fn test_paginator() {
        let conf = aws_config::load_from_env().await;
        let client = aws_sdk_ec2::Client::new(&conf);
        paginator_canary(client, 20).await.unwrap()
    }
}
