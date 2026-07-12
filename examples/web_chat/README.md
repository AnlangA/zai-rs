# Modern AI Chat Web Application

A production-quality web chat interface with streaming capabilities, perfect markdown rendering, and industry-leading user experience.

## 🌟 Features

### Core Functionality
- **Real-time Streaming**: Server-Sent Events (SSE) for smooth, real-time responses
- **Perfect Markdown Rendering**: Enhanced with syntax highlighting, code copy buttons, and beautiful styling
- **Industry-Leading Auto-Scroll**: Smart scroll detection with smooth animations
- **Session Management**: Persistent conversations with history and export capabilities
- **Think Mode**: Enhanced reasoning capabilities for complex queries

### Modern UI/UX
- **Responsive Design**: Mobile-first approach with seamless desktop experience
- **Dark/Light Theme**: Auto-detection with manual toggle
- **Minimal Whitespace**: Optimized layout maximizing content area
- **Smooth Animations**: CSS transitions and micro-interactions
- **Accessibility**: ARIA labels, keyboard navigation, screen reader support

### Advanced Features
- **Connection Management**: Automatic reconnection with exponential backoff
- **Rate Limiting**: Built-in protection against abuse
- **Error Handling**: Comprehensive error states and user feedback
- **Performance Optimized**: Lazy loading, debounced rendering, efficient DOM updates
- **Export Capabilities**: JSON, Markdown, Text, and HTML formats

## 🏗️ Architecture

### Backend Structure
```
src/
├── main.rs              # Application entry point
├── server.rs            # Server module
├── server/
│   ├── config.rs        # Configuration management
│   ├── error.rs         # Error types and handling
│   ├── state.rs         # Application state management
│   ├── utils.rs         # Utility functions
│   ├── models.rs        # Data model module
│   ├── models/          # Data models
│   │   ├── chat.rs      # Chat-related models
│   │   └── session.rs   # Session management models
│   ├── routes.rs        # Route module
│   └── routes/          # API routes
│       ├── chat.rs      # Chat endpoints
│       ├── health.rs    # Health checks
│       ├── index.rs     # Static file serving
│       └── static_files.rs # Static file serving
```

### Frontend Structure
```
static/
├── index.html           # Main HTML template
├── css/
│   └── main.css         # Modern CSS with custom properties
└── js/
    └── app.js           # Modular JavaScript application
```

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ 
- Node.js 18+ (for development)
- ZHIPU_API_KEY environment variable

### Installation
```bash
# Clone the repository
git clone <repository-url>
cd examples/web_chat

# Set up environment variables
export ZHIPU_API_KEY="your-api-key-here"
export PORT=3000  # Optional, defaults to 3000

# Build and run
cargo run --release
```

### Development
```bash
# Run in development mode
cargo run

# Run with hot reload (requires cargo-watch)
cargo watch -x run
```

## ⚙️ Configuration

Environment variables:
- `ZHIPU_API_KEY`: Required API key for Zhipu AI
- `PORT`: Server port (default: 3000)
- `CORS_ORIGINS`: Comma-separated list of allowed origins
- `SESSION_TIMEOUT`: Session timeout in seconds (default: 3600)
- `MAX_MESSAGES_PER_SESSION`: Maximum messages per session (default: 1000)
- `REQUEST_TIMEOUT`: Request timeout in seconds (default: 30)
- `ENABLE_LOGGING`: Enable request logging (default: true)

## 🎯 Perfect Markdown Rendering

The application features industry-leading markdown rendering with:

### Syntax Highlighting
- **Prism.js Integration**: Automatic language detection and highlighting
- **Copy Code Buttons**: One-click code copying with visual feedback
- **Language Badges**: Clear identification of programming languages
- **Line Numbers**: Optional line numbering for better code readability

### Enhanced Features
- **Math Support**: KaTeX integration for mathematical expressions
- **Mermaid Diagrams**: Flowcharts, sequence diagrams, and more
- **Task Lists**: Interactive checkboxes with proper styling
- **Tables**: Responsive tables with hover effects
- **Blockquotes**: Styled quotes with attribution support

### Custom Extensions
- **Code Block Headers**: Language identification and copy functionality
- **Smart Link Handling**: External links open in new tabs with security
- **Image Optimization**: Lazy loading and responsive images
- **Footnotes**: Proper footnote rendering and linking

## 📜 Industry-Leading Auto-Scroll

### Smart Scroll Detection
- **User Intent Detection**: Distinguishes between user scrolling and auto-scrolling
- **Threshold-based Triggers**: Configurable distance from bottom (default: 100px)
- **Momentum Detection**: Respects scroll momentum and touch gestures

### Smooth Animations
- **RequestAnimationFrame**: 60fps smooth scrolling animations
- **Easing Functions**: Natural acceleration and deceleration curves
- **Performance Optimized**: Minimal CPU usage during animations

### Adaptive Behavior
- **Variable Speed**: Adjusts scroll speed based on content length
- **Edge Prevention**: Prevents overscroll and bounce effects
- **Mobile Optimized**: Touch-friendly scroll behavior on mobile devices

## 🎨 Modern UI Design

### Design Principles
- **Minimal Whitespace**: Optimized layout maximizing content area
- **Visual Hierarchy**: Clear information architecture with proper contrast
- **Consistent Spacing**: Systematic spacing using CSS custom properties
- **Smooth Transitions**: Subtle animations enhancing user experience

### Responsive Design
- **Mobile-First**: Designed for mobile devices first, enhanced for desktop
- **Flexible Layouts**: CSS Grid and Flexbox for adaptive layouts
- **Touch Optimized**: Large touch targets and gesture support
- **Performance Focused**: Optimized for mobile network conditions

### Accessibility
- **WCAG 2.1 Compliance**: Meets accessibility standards
- **Keyboard Navigation**: Full keyboard support with visible focus indicators
- **Screen Reader Support**: Proper ARIA labels and semantic HTML
- **High Contrast Mode**: Support for high contrast preferences

## 🔧 API Endpoints

### Chat Endpoints
- `POST /api/chat/send` - Send regular chat message
- `POST /api/chat/stream` - Stream chat message (SSE)
- `GET /api/chat/history/:session_id` - Get chat history
- `POST /api/chat/clear/:session_id` - Clear chat history

### Session Management
- `POST /api/sessions` - Create new session
- `GET /api/sessions/:session_id` - Get session info
- `PUT /api/sessions/:session_id` - Update session
- `DELETE /api/sessions/:session_id` - Delete session

### Health & Status
- `GET /health` - Health check
- `GET /ready` - Readiness check
- `GET /live` - Liveness check

## 📊 Performance Optimizations

### Backend Optimizations
- **Connection Pooling**: Efficient database connection management
- **Async/Await**: Non-blocking I/O operations
- **Memory Efficient**: Streaming responses to minimize memory usage
- **Rate Limiting**: Built-in protection against abuse

### Frontend Optimizations
- **Code Splitting**: Modular JavaScript with dynamic imports
- **Lazy Loading**: Components loaded on demand
- **Debounced Rendering**: Optimized markdown rendering with debouncing
- **Virtual Scrolling**: Efficient handling of large conversation histories

### Network Optimizations
- **Compression**: Brotli compression for static assets
- **Caching**: Strategic caching of static resources
- **CDN Ready**: Designed for CDN deployment
- **Service Worker**: Offline support and caching strategies

## 🧪 Testing

### Backend Tests
```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin

# Run integration tests
cargo test --test integration
```

### Frontend Tests
```bash
# Run JavaScript tests (when implemented)
npm test

# Run end-to-end tests
npm run test:e2e
```

## 🚀 Deployment

### Docker Deployment
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/web_chat /usr/local/bin/web_chat
COPY --from=builder /app/static /app/static
WORKDIR /app
CMD ["web_chat"]
```

### Environment Setup
```bash
# Production environment variables
export ZHIPU_API_KEY="your-production-api-key"
export PORT=8080
export RUST_LOG=info
export CORS_ORIGINS="https://yourdomain.com"
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Zhipu AI** for providing the AI API
- **Axum** for the excellent web framework
- **Marked.js** for markdown rendering
- **Prism.js** for syntax highlighting
- **The Rust Community** for amazing tools and libraries

## 📞 Support

For support, please open an issue in the GitHub repository or contact the development team.

---

**Built with ❤️ using Rust and modern web technologies**
