load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def rust_repos():
    http_archive(
        name = "rules_rust",
        sha256 = "9d04e658878d23f4b00163a72da3db03ddb451273eb347df7d7c50838d698f49",
        urls = ["https://github.com/bazelbuild/rules_rust/releases/download/0.26.0/rules_rust-v0.26.0.tar.gz"],
    )
