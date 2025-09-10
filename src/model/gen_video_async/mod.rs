pub mod data;
pub mod video_model;
pub mod video_request;

pub use data::*;
pub use video_model::*;
pub use video_request::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::traits::{ModelName, VideoGen};
    use serde_json;

    // Mock model type for testing
    #[derive(Debug, Clone)]
    struct TestModel {}

    impl Into<String> for TestModel {
        fn into(self) -> String {
            "cogvideox-3".to_string()
        }
    }

    impl serde::Serialize for TestModel {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str("cogvideox-3")
        }
    }

    impl ModelName for TestModel {}
    impl VideoGen for TestModel {}

    #[test]
    fn test_video_body_prompt_only_serialization() {
        let model = TestModel {};
        let video_body = VideoBody::prompt_only(model, "A cat is playing with a ball.")
            .with_quality(VideoQuality::Quality)
            .with_audio(true)
            .with_size(VideoSize::Size1920x1080)
            .with_fps(Fps::Fps30);

        let json = serde_json::to_string(&video_body).unwrap();
        let expected = r#"{"model":"cogvideox-3","prompt":"A cat is playing with a ball.","quality":"quality","with_audio":true,"size":"1920x1080","fps":"fps30"}"#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_video_body_single_image_serialization() {
        let model = TestModel {};
        let video_body = VideoBody::with_single_image(
            model,
            "https://img.iplaysoft.com/wp-content/uploads/2019/free-images/free_stock_photo.jpg",
            "让画面动起来",
        )
        .with_quality(VideoQuality::Quality)
        .with_audio(true)
        .with_size(VideoSize::Size1920x1080)
        .with_fps(Fps::Fps30);

        let json = serde_json::to_string(&video_body).unwrap();
        let expected = r#"{"model":"cogvideox-3","image_url":["https://img.iplaysoft.com/wp-content/uploads/2019/free-images/free_stock_photo.jpg"],"prompt":"让画面动起来","quality":"quality","with_audio":true,"size":"1920x1080","fps":"fps30"}"#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_video_body_multiple_images_serialization() {
        let model = TestModel {};
        let image_urls = vec![
            "https://gd-hbimg.huaban.com/ccee58d77afe8f5e17a572246b1994f7e027657fe9e6-qD66In_fw1200webp",
            "https://gd-hbimg.huaban.com/cc2601d568a72d18d90b2cc7f1065b16b2d693f7fa3f7-hDAwNq_fw1200webp"
        ];
        let video_body = VideoBody::with_multiple_images(model, image_urls, "让画面动起来")
            .with_quality(VideoQuality::Quality)
            .with_audio(true)
            .with_size(VideoSize::Size1920x1080)
            .with_fps(Fps::Fps30);

        let json = serde_json::to_string(&video_body).unwrap();
        let expected = r#"{"model":"cogvideox-3","image_url":["https://gd-hbimg.huaban.com/ccee58d77afe8f5e17a572246b1994f7e027657fe9e6-qD66In_fw1200webp","https://gd-hbimg.huaban.com/cc2601d568a72d18d90b2cc7f1065b16b2d693f7fa3f7-hDAwNq_fw1200webp"],"prompt":"让画面动起来","quality":"quality","with_audio":true,"size":"1920x1080","fps":"fps30"}"#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_image_url_base64_serialization() {
        let image_url = ImageUrl::base64("data:image/png;base64,testbase64data");
        let json = serde_json::to_string(&image_url).unwrap();
        let expected = r#""data:image/png;base64,testbase64data""#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_image_url_single_url_serialization() {
        let image_url = ImageUrl::from_url("https://example.com/image.jpg");
        let json = serde_json::to_string(&image_url).unwrap();
        let expected = r#"["https://example.com/image.jpg"]"#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_image_url_multiple_urls_serialization() {
        let image_url = ImageUrl::from_two_urls(
            "https://example.com/image1.jpg",
            "https://example.com/image2.jpg",
        );
        let json = serde_json::to_string(&image_url).unwrap();
        let expected = r#"["https://example.com/image1.jpg","https://example.com/image2.jpg"]"#;

        assert_eq!(json, expected);
    }

    #[test]
    fn test_video_quality_serialization() {
        assert_eq!(
            serde_json::to_string(&VideoQuality::Speed).unwrap(),
            r#""speed""#
        );
        assert_eq!(
            serde_json::to_string(&VideoQuality::Quality).unwrap(),
            r#""quality""#
        );
    }

    #[test]
    fn test_video_size_serialization() {
        assert_eq!(
            serde_json::to_string(&VideoSize::Size1920x1080).unwrap(),
            r#""1920x1080""#
        );
        assert_eq!(
            serde_json::to_string(&VideoSize::Size3840x2160).unwrap(),
            r#""3840x2160""#
        );
    }

    #[test]
    fn test_fps_serialization() {
        assert_eq!(serde_json::to_string(&Fps::Fps30).unwrap(), r#""fps30""#);
        assert_eq!(serde_json::to_string(&Fps::Fps60).unwrap(), r#""fps60""#);
    }

    #[test]
    fn test_video_duration_serialization() {
        assert_eq!(
            serde_json::to_string(&VideoDuration::Duration5).unwrap(),
            r#""duration5""#
        );
        assert_eq!(
            serde_json::to_string(&VideoDuration::Duration10).unwrap(),
            r#""duration10""#
        );
    }

    #[test]
    fn test_video_body_builder_methods() {
        let model = TestModel {};
        let video_body = VideoBody::new(model)
            .with_prompt("Test prompt")
            .with_quality(VideoQuality::Speed)
            .with_audio(false)
            .with_size(VideoSize::Size1280x720)
            .with_fps(Fps::Fps60)
            .with_duration(VideoDuration::Duration10);

        let json = serde_json::to_string(&video_body).unwrap();
        assert!(json.contains(r#""prompt":"Test prompt""#));
        assert!(json.contains(r#""quality":"speed""#));
        assert!(json.contains(r#""with_audio":false"#));
        assert!(json.contains(r#""size":"1280x720""#));
        assert!(json.contains(r#""fps":"fps60""#));
        assert!(json.contains(r#""duration":"duration10""#));
    }

    #[test]
    fn test_video_body_skip_none_fields() {
        let model = TestModel {};
        let video_body = VideoBody::new(model).with_prompt("Test prompt");

        let json = serde_json::to_string(&video_body).unwrap();
        assert!(json.contains(r#""prompt":"Test prompt""#));
        assert!(!json.contains("quality"));
        assert!(!json.contains("with_audio"));
        assert!(!json.contains("size"));
        assert!(!json.contains("fps"));
        assert!(!json.contains("duration"));
    }
}
