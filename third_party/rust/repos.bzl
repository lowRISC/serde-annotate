load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def rust_repos():
    http_archive(
        name = "rules_rust",
        sha256 = "fe8d7e62d093e10d8f87b9fd26e4ee2b8d64667ab6827f41fb38eda640233e0f",
        strip_prefix = "rules_rust-add-missing-rv32-shas-20221031_01",
        url = "https://github.com/lowRISC/rules_rust/archive/refs/tags/add-missing-rv32-shas-20221031_01.tar.gz",
    )
