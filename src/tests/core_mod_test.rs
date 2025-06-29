#[cfg(test)]
mod tests {
    use crate::core::{
        ArchiveStats, CompressionAlgorithm, EncryptionAlgorithm, FeatureFlags, FileType,
        RZX_MAGIC, RZX_VERSION,
    };
    

    #[test]
    fn test_compression_algorithm_from_str() {
        assert_eq!(
            CompressionAlgorithm::from_str("none"),
            Some(CompressionAlgorithm::None)
        );
        assert_eq!(
            CompressionAlgorithm::from_str("deflate"),
            Some(CompressionAlgorithm::Deflate)
        );
        assert_eq!(
            CompressionAlgorithm::from_str("lzma"),
            Some(CompressionAlgorithm::Lzma)
        );
        assert_eq!(
            CompressionAlgorithm::from_str("zstd"),
            Some(CompressionAlgorithm::Zstd)
        );
        assert_eq!(CompressionAlgorithm::from_str("unknown"), None);
    }

    #[test]
    fn test_compression_algorithm_as_str() {
        assert_eq!(CompressionAlgorithm::None.as_str(), "none");
        assert_eq!(CompressionAlgorithm::Deflate.as_str(), "deflate");
        assert_eq!(CompressionAlgorithm::Lzma.as_str(), "lzma");
        assert_eq!(CompressionAlgorithm::Zstd.as_str(), "zstd");
    }

    #[test]
    fn test_encryption_algorithm_from_str() {
        assert_eq!(
            EncryptionAlgorithm::from_str("none"),
            Some(EncryptionAlgorithm::None)
        );
        assert_eq!(
            EncryptionAlgorithm::from_str("aes256gcm"),
            Some(EncryptionAlgorithm::Aes256Gcm)
        );
        assert_eq!(EncryptionAlgorithm::from_str("unknown"), None);
    }

    #[test]
    fn test_encryption_algorithm_as_str() {
        assert_eq!(EncryptionAlgorithm::None.as_str(), "none");
        assert_eq!(EncryptionAlgorithm::Aes256Gcm.as_str(), "aes256gcm");
    }

    #[test]
    fn test_encryption_algorithm_from_u8() {
        assert_eq!(EncryptionAlgorithm::from_u8(0), Some(EncryptionAlgorithm::None));
        assert_eq!(
            EncryptionAlgorithm::from_u8(1),
            Some(EncryptionAlgorithm::Aes256Gcm)
        );
        assert_eq!(EncryptionAlgorithm::from_u8(99), None);
    }

    #[test]
    fn test_feature_flags() {
        let mut flags = FeatureFlags::NONE;
        assert!(!flags.has_flag(FeatureFlags::ENCRYPTED));
        assert!(!flags.has_flag(FeatureFlags::COMPRESSED));

        flags.set_flag(FeatureFlags::ENCRYPTED);
        assert!(flags.has_flag(FeatureFlags::ENCRYPTED));
        assert!(!flags.has_flag(FeatureFlags::COMPRESSED));

        flags.set_flag(FeatureFlags::COMPRESSED);
        assert!(flags.has_flag(FeatureFlags::ENCRYPTED));
        assert!(flags.has_flag(FeatureFlags::COMPRESSED));

        flags.clear_flag(FeatureFlags::ENCRYPTED);
        assert!(!flags.has_flag(FeatureFlags::ENCRYPTED));
        assert!(flags.has_flag(FeatureFlags::COMPRESSED));
    }

    #[test]
    fn test_file_type_from_extension() {
        assert_eq!(FileType::from_extension("txt"), FileType::Text);
        assert_eq!(FileType::from_extension("JPG"), FileType::Image);
        assert_eq!(FileType::from_extension("mp3"), FileType::Audio);
        assert_eq!(FileType::from_extension("mkv"), FileType::Video);
        assert_eq!(FileType::from_extension("zip"), FileType::Archive);
        assert_eq!(FileType::from_extension("pdf"), FileType::Document);
        assert_eq!(FileType::from_extension("exe"), FileType::Binary);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
    }

    #[test]
    fn test_file_type_best_compression() {
        assert_eq!(
            FileType::Text.best_compression(),
            CompressionAlgorithm::Lzma
        );
        assert_eq!(
            FileType::Document.best_compression(),
            CompressionAlgorithm::Lzma
        );
        assert_eq!(
            FileType::Binary.best_compression(),
            CompressionAlgorithm::Zstd
        );
        assert_eq!(
            FileType::Image.best_compression(),
            CompressionAlgorithm::None
        );
        assert_eq!(
            FileType::Audio.best_compression(),
            CompressionAlgorithm::None
        );
        assert_eq!(
            FileType::Video.best_compression(),
            CompressionAlgorithm::None
        );
        assert_eq!(
            FileType::Archive.best_compression(),
            CompressionAlgorithm::None
        );
        assert_eq!(
            FileType::Unknown.best_compression(),
            CompressionAlgorithm::Deflate
        );
    }

    #[test]
    fn test_archive_stats_new() {
        let stats = ArchiveStats::new();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_dirs, 0);
        assert_eq!(stats.uncompressed_size, 0);
        assert_eq!(stats.compressed_size, 0);
        assert_eq!(stats.compression_ratio, 0.0);
        assert!(stats.created_at.is_some());
        assert!(stats.modified_at.is_some());
    }

    #[test]
    fn test_archive_stats_add_file() {
        let mut stats = ArchiveStats::new();
        stats.add_file(100, 50);
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.uncompressed_size, 100);
        assert_eq!(stats.compressed_size, 50);
        assert_eq!(stats.compression_ratio, 0.5); // 1 - (50/100)
        assert!(stats.modified_at.is_some());

        stats.add_file(200, 100);
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.uncompressed_size, 300);
        assert_eq!(stats.compressed_size, 150);
        assert_eq!(stats.compression_ratio, 0.5); // 1 - (150/300)
    }

    #[test]
    fn test_archive_stats_add_directory() {
        let mut stats = ArchiveStats::new();
        stats.add_directory();
        assert_eq!(stats.total_dirs, 1);
        assert!(stats.modified_at.is_some());
    }

    #[test]
    fn test_archive_stats_update_compression_ratio_valid() {
        let mut stats = ArchiveStats::new();
        stats.uncompressed_size = 1000;
        stats.compressed_size = 250;
        stats.update_compression_ratio();
        assert_eq!(stats.compression_ratio, 0.75);
    }

    #[test]
    fn test_archive_stats_update_compression_ratio_zero_uncompressed() {
        let mut stats = ArchiveStats::new();
        stats.uncompressed_size = 0;
        stats.compressed_size = 100;
        stats.update_compression_ratio();
        assert_eq!(stats.compression_ratio, 0.0); // Should remain 0.0 if uncompressed_size is 0
    }

    #[test]
    fn test_rzx_constants() {
        assert_eq!(RZX_VERSION, 1);
        assert_eq!(RZX_MAGIC, [b'R', b'z', b'X', 0]);
    }
}
