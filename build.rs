fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[
                "proto/apps/v0/common.proto",
                "proto/apps/v0/tool.proto",
                "proto/apps/v0/channel.proto",
                "proto/apps/v0/gateway.proto",
                "proto/apps/v0/comm.proto",
                "proto/apps/v0/ui.proto",
                "proto/apps/v0/schedule.proto",
            ],
            &["."],
        )?;
    Ok(())
}
