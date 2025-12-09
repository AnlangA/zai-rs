# Real-time Audio and Video Examples

This directory contains examples demonstrating how to use the GLM-Realtime API for real-time audio and video conversations with AI models.

## Examples

### Audio Chat (`audio_chat.rs`)

This example shows how to:

- Connect to the GLM-Realtime API for audio conversations
- Configure a session for voice interactions
- Send audio data to the model
- Receive and process audio responses
- Handle various server events

To run this example:

```bash
cargo run --example audio_chat
```

Note: For this example to work properly, you would need to:
1. Set the `ZHIPU_API_KEY` environment variable with your API key
2. Have audio files in the `audio_samples/` directory (optional, as the example includes a fallback)
3. In a real application, implement audio capture from a microphone and audio playback

### Video Chat (`video_chat.rs`)

This example demonstrates how to:

- Connect to the GLM-Realtime API for video conversations
- Configure a session for video interactions
- Send video frames to the model
- Receive video-aware responses
- Handle function calling during video conversations

To run this example:

```bash
cargo run --example video_chat
```

Note: For this example to work properly, you would need to:
1. Set the `ZHIPU_API_KEY` environment variable with your API key
2. Have image files in the `video_samples/` directory (optional, as the example includes a fallback)
3. In a real application, implement video capture from a camera and video display

## Implementation Notes

These examples provide a foundation for building real-time audio/video applications. In a production environment, you would need to:

1. Implement proper audio capture from a microphone using libraries like `cpal` or `rodio`
2. Implement proper video capture from a camera using libraries like `opencv`
3. Handle audio playback and video display in a user-friendly way
4. Add proper error handling and reconnection logic
5. Consider audio format conversion as needed
6. Implement UI controls for starting/stopping conversations

## API Key Setup

To use these examples, you need to set your Zhipu AI API key as an environment variable:

```bash
export ZHIPU_API_KEY=your_api_key_here
```

Or on Windows:

```powershell
$env:ZHIPU_API_KEY = "your_api_key_here"
```

## Dependencies

These examples rely on the following crates already included in `zai-rs`:
- `tokio-tungstenite` for WebSocket connections
- `serde` for JSON serialization/deserialization
- `base64` for audio/video encoding
- `uuid` for generating event IDs

## GLM-Realtime Models

The examples use different GLM-Realtime models:

- `GLMRealtimeFlash`: 9B parameter model, lower cost
  - Audio: 0.18元/分钟
  - Video: 1.2元/分钟

- `GLMRealtimeAir`: 32B parameter model, higher quality
  - Audio: 0.3元/分钟
  - Video: 2.1元/分钟

For more information about the GLM-Realtime API, see the [official documentation](https://docs.bigmodel.cn/cn/guide/models/sound-and-video/glm-realtime).