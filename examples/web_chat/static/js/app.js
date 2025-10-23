/**
 * Modern AI Chat Application
 * Perfect markdown rendering with industry-leading auto-scroll
 */

// Application state
const App = {
    // Configuration
    config: {
        apiEndpoint: '/api',
        streamingEndpoint: '/api/chat/stream',
        maxMessageLength: 10000,
        autoScrollThreshold: 100, // pixels from bottom
        typingIndicatorDelay: 500, // ms
        messageAnimationDuration: 300, // ms
        markdownRenderDelay: 50, // debounce delay for markdown rendering
        scrollAnimationDuration: 200, // ms
        connectionCheckInterval: 30000, // 30 seconds
        maxReconnectAttempts: 5,
        reconnectDelay: 1000, // ms
    },

    // State
    state: {
        currentSessionId: null,
        isStreaming: false,
        isConnected: false,
        thinkMode: false,
        autoScroll: true,
        soundEnabled: false,
        theme: 'auto',
        temperature: 0.7,
        maxTokens: 2048,
        selectedModel: 'GLM4_6',
        reconnectAttempts: 0,
        messageHistory: [],
        conversationHistory: [],
    },

    // DOM elements cache
    elements: {},

    // Markdown renderer
    markdownRenderer: null,

    // Auto-scroll controller
    scrollController: null,

    // Connection manager
    connectionManager: null,

    // rAF batching flag for streaming renders
    _streamRafScheduled: false,

    // Compute cutoff index for stable (closed) markdown region
    computeStableEnd(markdown) {
        const fenceRe = /```/g;
        let m, count = 0, lastPos = -1;
        while ((m = fenceRe.exec(markdown)) !== null) { count++; lastPos = m.index; }
        if (count % 2 === 0) return markdown.length;
        return lastPos >= 0 ? lastPos : 0;
    },

    // rAF batching flag for streaming renders
    _streamRafScheduled: false,

    // Initialize application
    async init() {
        console.log('🚀 Initializing Modern AI Chat Application');
        
        try {
            // Cache DOM elements
            this.cacheElements();
            
            // Initialize services
            await this.initializeServices();
            
            // Setup event listeners
            this.setupEventListeners();
            
            // Load saved state
            await this.loadState();
            
            // Initialize UI
            await this.initializeUI();
            
            // Check connection
            await this.checkConnection();
            
            // Hide loading screen
            this.hideLoadingScreen();
            
            console.log('✅ Application initialized successfully');
            
        } catch (error) {
            console.error('❌ Failed to initialize application:', error);
            this.showError('Failed to initialize application. Please refresh the page.');
        }
    },

    // Cache DOM elements for performance
    cacheElements() {
        this.elements = {
            // Main containers
            appContainer: document.getElementById('appContainer'),
            loadingScreen: document.getElementById('loadingScreen'),
            messagesContainer: document.getElementById('messagesContainer'),
            
            // Header elements
            menuToggle: document.getElementById('menuToggle'),
            themeToggle: document.getElementById('themeToggle'),
            settingsToggle: document.getElementById('settingsToggle'),
            connectionStatus: document.getElementById('connectionStatus'),
            statusIndicator: document.getElementById('statusIndicator'),
            statusText: document.getElementById('statusText'),
            
            // Sidebar elements
            sidebar: document.getElementById('sidebar'),
            newChatBtn: document.getElementById('newChatBtn'),
            conversationList: document.getElementById('conversationList'),
            exportBtn: document.getElementById('exportBtn'),
            
            // Chat elements
            welcomeMessage: document.getElementById('welcomeMessage'),
            typingIndicator: document.getElementById('typingIndicator'),
            messageInput: document.getElementById('messageInput'),
            sendBtn: document.getElementById('sendBtn'),
            thinkToggle: document.getElementById('thinkToggle'),
            attachBtn: document.getElementById('attachBtn'),
            tokenCounter: document.getElementById('tokenCounter'),
            modelSelector: document.getElementById('modelSelector'),
            
            // Settings elements
            settingsPanel: document.getElementById('settingsPanel'),
            closeSettings: document.getElementById('closeSettings'),
            overlay: document.getElementById('overlay'),
            themeSelectorSettings: document.getElementById('themeSelectorSettings'),
            temperatureSlider: document.getElementById('temperatureSlider'),
            temperatureValue: document.getElementById('temperatureValue'),
            maxTokensSlider: document.getElementById('maxTokensSlider'),
            maxTokensValue: document.getElementById('maxTokensValue'),
            autoScrollToggle: document.getElementById('autoScrollToggle'),
            soundToggle: document.getElementById('soundToggle'),
        };
    },

    // Initialize services
    async initializeServices() {
        // Initialize markdown renderer
        this.markdownRenderer = new MarkdownRenderer();
        await this.markdownRenderer.init();
        
        // Initialize scroll controller
        this.scrollController = new ScrollController(this.elements.messagesContainer, {
            threshold: this.config.autoScrollThreshold,
            animationDuration: this.config.scrollAnimationDuration,
        });
        
        // Initialize connection manager
        this.connectionManager = new ConnectionManager({
            checkInterval: this.config.connectionCheckInterval,
            maxReconnectAttempts: this.config.maxReconnectAttempts,
            reconnectDelay: this.config.reconnectDelay,
        });
    },

    // Setup event listeners
    setupEventListeners() {
        // Header events
        this.elements.menuToggle?.addEventListener('click', () => this.toggleSidebar());
        this.elements.themeToggle?.addEventListener('click', () => this.toggleTheme());
        this.elements.settingsToggle?.addEventListener('click', () => this.toggleSettings());
        
        // Sidebar events
        this.elements.newChatBtn?.addEventListener('click', () => this.startNewChat());
        this.elements.exportBtn?.addEventListener('click', () => this.exportConversation());
        
        // Chat events
        this.elements.messageInput?.addEventListener('input', () => this.handleInputChange());
        this.elements.messageInput?.addEventListener('keydown', (e) => this.handleInputKeydown(e));
        this.elements.sendBtn?.addEventListener('click', () => this.sendMessage());
        this.elements.thinkToggle?.addEventListener('click', () => this.toggleThinkMode());
        
        // Settings events
        this.elements.closeSettings?.addEventListener('click', () => this.closeSettings());
        this.elements.overlay?.addEventListener('click', () => this.closeSettings());
        this.elements.themeSelectorSettings?.addEventListener('change', (e) => this.setTheme(e.target.value));
        this.elements.temperatureSlider?.addEventListener('input', (e) => this.updateTemperature(e.target.value));
        this.elements.maxTokensSlider?.addEventListener('input', (e) => this.updateMaxTokens(e.target.value));
        this.elements.autoScrollToggle?.addEventListener('change', (e) => this.setAutoScroll(e.target.checked));
        this.elements.soundToggle?.addEventListener('change', (e) => this.setSoundEnabled(e.target.checked));
        
        // Window events
        window.addEventListener('resize', () => this.handleResize());
        window.addEventListener('beforeunload', () => this.saveState());
        
        // Connection events
        this.connectionManager.on('connected', () => this.handleConnectionConnected());
        this.connectionManager.on('disconnected', () => this.handleConnectionDisconnected());
        this.connectionManager.on('reconnecting', (attempt) => this.handleConnectionReconnecting(attempt));
        
        // Auto-scroll events
        this.scrollController.on('scroll', () => this.handleScroll());
        this.scrollController.on('nearBottom', () => this.handleNearBottom());
        this.scrollController.on('awayFromBottom', () => this.handleAwayFromBottom());
    },

    // Load saved state
    async loadState() {
        try {
            // Load from localStorage
            const savedState = localStorage.getItem('aiChatState');
            if (savedState) {
                const parsed = JSON.parse(savedState);
                this.state = { ...this.state, ...parsed };
            }
            
            // Load session ID
            this.state.currentSessionId = localStorage.getItem('chatSessionId');
            
            console.log('💾 State loaded successfully');
        } catch (error) {
            console.warn('⚠️ Failed to load saved state:', error);
        }
    },

    // Save state
    saveState() {
        try {
            const stateToSave = {
                theme: this.state.theme,
                thinkMode: this.state.thinkMode,
                autoScroll: this.state.autoScroll,
                soundEnabled: this.state.soundEnabled,
                temperature: this.state.temperature,
                maxTokens: this.state.maxTokens,
                selectedModel: this.state.selectedModel,
            };
            
            localStorage.setItem('aiChatState', JSON.stringify(stateToSave));
            localStorage.setItem('chatSessionId', this.state.currentSessionId || '');
            
            console.log('💾 State saved successfully');
        } catch (error) {
            console.warn('⚠️ Failed to save state:', error);
        }
    },

    // Initialize UI
    async initializeUI() {
        // Apply theme
        this.applyTheme(this.state.theme);
        
        // Set initial values
        this.elements.temperatureSlider && (this.elements.temperatureSlider.value = this.state.temperature);
        this.elements.temperatureValue && (this.elements.temperatureValue.textContent = this.state.temperature);
        this.elements.maxTokensSlider && (this.elements.maxTokensSlider.value = this.state.maxTokens);
        this.elements.maxTokensValue && (this.elements.maxTokensValue.textContent = this.state.maxTokens);
        this.elements.autoScrollToggle && (this.elements.autoScrollToggle.checked = this.state.autoScroll);
        this.elements.soundToggle && (this.elements.soundToggle.checked = this.state.soundEnabled);
        this.elements.modelSelector && (this.elements.modelSelector.value = this.state.selectedModel);
        
        // Update think mode button
        this.updateThinkModeButton();
        
        // Load conversation history
        await this.loadConversationHistory();
        
        // Show welcome message if no current session
        if (!this.state.currentSessionId) {
            this.showWelcomeMessage();
        } else {
            await this.loadCurrentConversation();
        }
    },

    // Hide loading screen
    hideLoadingScreen() {
        if (this.elements.loadingScreen) {
            this.elements.loadingScreen.classList.add('hidden');
            setTimeout(() => {
                this.elements.loadingScreen.style.display = 'none';
                this.elements.appContainer?.classList.add('loaded');
            }, 300);
        }
    },

    // Show error message
    showError(message) {
        console.error('❌ Error:', message);
        // In a real app, you'd show a proper error UI
        alert(`Error: ${message}`);
    },

    // Connection management
    async checkConnection() {
        try {
            const response = await fetch('/health');
            const data = await response.json();
            
            if (response.ok && data.status === 'healthy') {
                this.handleConnectionConnected();
            } else {
                this.handleConnectionDisconnected();
            }
        } catch (error) {
            console.warn('⚠️ Connection check failed:', error);
            this.handleConnectionDisconnected();
        }
    },

    handleConnectionConnected() {
        this.state.isConnected = true;
        this.state.reconnectAttempts = 0;
        
        if (this.elements.statusIndicator) {
            this.elements.statusIndicator.classList.add('connected');
            this.elements.statusIndicator.classList.remove('connecting', 'error');
        }
        
        if (this.elements.statusText) {
            this.elements.statusText.textContent = 'Connected';
        }
        
        this.elements.sendBtn?.removeAttribute('disabled');
        console.log('🔗 Connected to server');
    },

    handleConnectionDisconnected() {
        this.state.isConnected = false;
        
        if (this.elements.statusIndicator) {
            this.elements.statusIndicator.classList.add('error');
            this.elements.statusIndicator.classList.remove('connected', 'connecting');
        }
        
        if (this.elements.statusText) {
            this.elements.statusText.textContent = 'Disconnected';
        }
        
        this.elements.sendBtn?.setAttribute('disabled', 'true');
        console.warn('🔌 Disconnected from server');
    },

    handleConnectionReconnecting(attempt) {
        this.state.reconnectAttempts = attempt;
        
        if (this.elements.statusIndicator) {
            this.elements.statusIndicator.classList.add('connecting');
            this.elements.statusIndicator.classList.remove('connected', 'error');
        }
        
        if (this.elements.statusText) {
            this.elements.statusText.textContent = `Reconnecting... (${attempt}/${this.config.maxReconnectAttempts})`;
        }
        
        console.log(`🔄 Reconnecting... Attempt ${attempt}`);
    },

    // Theme management
    toggleTheme() {
        const themes = ['light', 'dark', 'auto'];
        const currentIndex = themes.indexOf(this.state.theme);
        const nextIndex = (currentIndex + 1) % themes.length;
        const nextTheme = themes[nextIndex];
        
        this.setTheme(nextTheme);
    },

    setTheme(theme) {
        this.state.theme = theme;
        this.applyTheme(theme);
        this.saveState();
    },

    applyTheme(theme) {
        const html = document.documentElement;
        
        if (theme === 'auto') {
            html.removeAttribute('data-theme');
        } else {
            html.setAttribute('data-theme', theme);
        }
        
        // Update theme toggle button
        this.updateThemeToggleButton();
    },

    updateThemeToggleButton() {
        // This would update the theme toggle button appearance
        // Implementation depends on your UI design
    },

    // Sidebar management
    toggleSidebar() {
        const sidebar = this.elements.sidebar;
        if (sidebar) {
            sidebar.classList.toggle('open');
            const isOpen = sidebar.classList.contains('open');
            this.elements.menuToggle?.setAttribute('aria-expanded', isOpen);
        }
    },

    // Settings management
    toggleSettings() {
        const isOpen = this.elements.settingsPanel?.classList.contains('open');
        if (isOpen) {
            this.closeSettings();
        } else {
            this.openSettings();
        }
    },

    openSettings() {
        this.elements.settingsPanel?.classList.add('open');
        this.elements.overlay?.classList.add('visible');
        this.elements.settingsToggle?.setAttribute('aria-expanded', 'true');
    },

    closeSettings() {
        this.elements.settingsPanel?.classList.remove('open');
        this.elements.overlay?.classList.remove('visible');
        this.elements.settingsToggle?.setAttribute('aria-expanded', 'false');
    },

    // Temperature and token management
    updateTemperature(value) {
        this.state.temperature = parseFloat(value);
        if (this.elements.temperatureValue) {
            this.elements.temperatureValue.textContent = value;
        }
        this.saveState();
    },

    updateMaxTokens(value) {
        this.state.maxTokens = parseInt(value);
        if (this.elements.maxTokensValue) {
            this.elements.maxTokensValue.textContent = value;
        }
        this.saveState();
    },

    setAutoScroll(enabled) {
        this.state.autoScroll = enabled;
        this.scrollController.setAutoScroll(enabled);
        this.saveState();
    },

    setSoundEnabled(enabled) {
        this.state.soundEnabled = enabled;
        this.saveState();
    },

    // Think mode
    toggleThinkMode() {
        this.state.thinkMode = !this.state.thinkMode;
        this.updateThinkModeButton();
        this.saveState();
    },

    updateThinkModeButton() {
        const button = this.elements.thinkToggle;
        if (button) {
            button.classList.toggle('active', this.state.thinkMode);
            button.setAttribute('aria-pressed', this.state.thinkMode);
            button.title = this.state.thinkMode ? 'Think mode: ON' : 'Think mode: OFF';
        }
    },

    // Input handling
    handleInputChange() {
        const input = this.elements.messageInput;
        const sendBtn = this.elements.sendBtn;
        
        if (input && sendBtn) {
            const hasContent = input.value.trim().length > 0;
            sendBtn.disabled = !hasContent || this.state.isStreaming;
            
            // Auto-resize textarea
            this.autoResizeTextarea(input);
            
            // Update token counter
            this.updateTokenCounter(input.value);
        }
    },

    handleInputKeydown(event) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            this.sendMessage();
        }
    },

    autoResizeTextarea(textarea) {
        textarea.style.height = 'auto';
        textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
    },

    updateTokenCounter(text) {
        // Simple token estimation (1 token ≈ 4 characters)
        const estimatedTokens = Math.ceil(text.length / 4);
        const maxTokens = this.state.maxTokens;
        
        if (this.elements.tokenCounter) {
            this.elements.tokenCounter.textContent = `${estimatedTokens}/${maxTokens}`;
            
            // Color coding
            if (estimatedTokens > maxTokens * 0.9) {
                this.elements.tokenCounter.style.color = 'var(--color-error)';
            } else if (estimatedTokens > maxTokens * 0.7) {
                this.elements.tokenCounter.style.color = 'var(--color-warning)';
            } else {
                this.elements.tokenCounter.style.color = 'var(--color-text-muted)';
            }
        }
    },

    // Message sending
    async sendMessage() {
        const input = this.elements.messageInput;
        const message = input?.value.trim();
        
        if (!message || this.state.isStreaming || !this.state.isConnected) {
            return;
        }
        
        // Clear input
        if (input) {
            input.value = '';
            this.handleInputChange();
        }
        
        // Add user message to UI
        this.addMessage(message, 'user');
        
        // Show typing indicator
        this.showTypingIndicator();
        
        // Create session if needed
        if (!this.state.currentSessionId) {
            this.state.currentSessionId = await this.createSession();
        }
        
        // Send message
        try {
            const request = {
                message: message,
                session_id: this.state.currentSessionId,
                think: this.state.thinkMode,
                temperature: this.state.temperature,
                max_tokens: this.state.maxTokens,
                model: this.state.selectedModel,
                stream: true,
            };
            
            await this.streamMessage(request);
            
        } catch (error) {
            console.error('❌ Failed to send message:', error);
            this.addMessage('抱歉，发送消息时出现错误。请稍后重试。', 'assistant', true);
        } finally {
            this.hideTypingIndicator();
        }
    },

    // Add message to UI
    addMessage(content, role, isError = false) {
        const message = {
            id: this.generateMessageId(),
            role: role,
            content: content,
            timestamp: new Date(),
            isError: isError,
        };
        
        this.state.messageHistory.push(message);
        
        const messageElement = this.createMessageElement(message);
        this.elements.messagesContainer?.appendChild(messageElement);
        
        // Auto-scroll
        if (this.state.autoScroll) {
            this.scrollController.scrollToBottom();
        }
        
        // Play sound if enabled
        if (this.state.soundEnabled && role === 'assistant') {
            this.playMessageSound();
        }
    },

    createMessageElement(message) {
        const element = document.createElement('div');
        element.className = `message ${message.role}-message`;
        element.dataset.messageId = message.id;
        
        const avatar = document.createElement('div');
        avatar.className = 'message-avatar';
        avatar.innerHTML = message.role === 'user' ? '👤' : '🤖';
        
        const content = document.createElement('div');
        content.className = 'message-content';
        
        const bubble = document.createElement('div');
        bubble.className = 'message-bubble';
        
        const text = document.createElement('div');
        text.className = 'message-text';
        
        if (message.isError) {
            text.classList.add('error-message');
            text.textContent = message.content;
        } else if (message.role === 'assistant') {
            // Render markdown for assistant messages
            text.innerHTML = this.markdownRenderer.render(message.content);
        } else {
            text.textContent = message.content;
        }
        
        const time = document.createElement('div');
        time.className = 'message-time';
        time.textContent = this.formatTime(message.timestamp);
        
        bubble.appendChild(text);
        bubble.appendChild(time);
        content.appendChild(bubble);
        element.appendChild(avatar);
        element.appendChild(content);
        
        return element;
    },

    // Streaming message handling
    async streamMessage(request) {
        this.state.isStreaming = true;
        
        try {
            const response = await fetch(this.config.streamingEndpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Accept': 'text/event-stream',
                },
                body: JSON.stringify(request),
            });
            
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }
            
            // Create streaming message element
            const streamingElement = this.createStreamingMessageElement();
            this.elements.messagesContainer?.appendChild(streamingElement);
            // Stream rendering should avoid heavy highlighting; switch renderer to streaming mode
            this.markdownRenderer?.setStreaming?.(true);
            
            // Process streaming response
            const reader = response.body.getReader();
            const decoder = new TextDecoder();
            let buffer = '';
            
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                
                buffer += decoder.decode(value, { stream: true });
                
                // Process SSE events
                const lines = buffer.split('\n');
                buffer = lines.pop() || '';
                
                for (const line of lines) {
                    if (line.startsWith('data: ')) {
                        try {
                            const data = JSON.parse(line.slice(6));
                            await this.handleStreamChunk(data, streamingElement);
                        } catch (error) {
                            console.warn('⚠️ Failed to parse stream chunk:', error);
                        }
                    }
                }
            }
            
        } catch (error) {
            console.error('❌ Streaming error:', error);
            throw error;
        } finally {
            this.state.isStreaming = false;
        }
    },

    createStreamingMessageElement() {
        const element = document.createElement('div');
        element.className = 'message assistant-message streaming';
        element.dataset.messageId = this.generateMessageId();
        
        const avatar = document.createElement('div');
        avatar.className = 'message-avatar';
        avatar.innerHTML = '🤖';
        
        const content = document.createElement('div');
        content.className = 'message-content';
        
        const bubble = document.createElement('div');
        bubble.className = 'message-bubble';
        
        const text = document.createElement('div');
        text.className = 'message-text streaming-text';
        text.setAttribute('data-raw-content', '');
        text.setAttribute('data-stable-len', '0');
        text.innerHTML = '<div class="stable"></div><div class="tail"><span class="streaming-cursor">▌</span></div>';
        
        const time = document.createElement('div');
        time.className = 'message-time';
        time.textContent = 'Typing...';
        
        bubble.appendChild(text);
        bubble.appendChild(time);
        content.appendChild(bubble);
        element.appendChild(avatar);
        element.appendChild(content);
        
        // Store references for streaming updates
        element.streamingText = text;
        element.streamingTime = time;
        
        return element;
    },

    // Schedule a micro-batched render for the current streaming element (partial highlight for closed blocks)
    scheduleStreamingRender(streamingElement) {
        if (this._streamRafScheduled) return;
        this._streamRafScheduled = true;
        requestAnimationFrame(() => {
            this._streamRafScheduled = false;
            const textEl = streamingElement?.streamingText;
            if (!textEl) return;

            // Ensure stable/tail containers
            let stable = textEl.querySelector('.stable');
            let tail = textEl.querySelector('.tail');
            if (!stable || !tail) {
                textEl.innerHTML = '<div class="stable"></div><div class="tail"><span class="streaming-cursor">\u258c</span></div>';
                stable = textEl.querySelector('.stable');
                tail = textEl.querySelector('.tail');
            }

            const raw = textEl.getAttribute('data-raw-content') || '';

            // Compute stable end by code fence balance
            const stableEnd = this.computeStableEnd(raw);
            const prevStableLen = parseInt(textEl.getAttribute('data-stable-len') || '0', 10);

            if (stableEnd !== prevStableLen) {
                const stableMd = raw.slice(0, stableEnd);
                // Render stable with highlighting (temporarily disable streaming mode)
                const wasStreaming = !!this.markdownRenderer.streaming;
                this.markdownRenderer.setStreaming(false);
                stable.innerHTML = this.markdownRenderer.render(stableMd);
                this.markdownRenderer.setStreaming(wasStreaming);
                try {
                    const prism = this.markdownRenderer?.prism;
                    if (prism) prism.highlightAllUnder(stable);
                } catch (_) {}
                textEl.setAttribute('data-stable-len', String(stableEnd));
            }

            // Render tail without highlighting
            const tailMd = raw.slice(stableEnd);
            tail.innerHTML = this.markdownRenderer.render(tailMd) + '<span class="streaming-cursor">\u258c</span>';
        });
    },

    async handleStreamChunk(chunk, streamingElement) {
        if (!streamingElement.streamingText) return;

        // Update content (no highlighting during stream)
        if (chunk.content) {
            const currentContent = streamingElement.streamingText.getAttribute('data-raw-content') || '';
            const newContent = currentContent + chunk.content;
            streamingElement.streamingText.setAttribute('data-raw-content', newContent);
            this.scheduleStreamingRender(streamingElement);

            // Auto-scroll
            if (this.state.autoScroll) {
                this.scrollController.scrollToBottom();
            }
        }

        // Update session ID
        if (chunk.session_id) {
            this.state.currentSessionId = chunk.session_id;
        }

        // Handle completion
        if (chunk.done) {
            // Final render with highlighting
            const rawContent = streamingElement.streamingText.getAttribute('data-raw-content') || '';
            this.markdownRenderer?.setStreaming?.(false);
            streamingElement.streamingText.innerHTML = this.markdownRenderer.render(rawContent);
            streamingElement.streamingTime.textContent = this.formatTime(new Date());

            // Remove streaming class
            streamingElement.classList.remove('streaming');

            // Add to message history
            this.state.messageHistory.push({
                id: streamingElement.dataset.messageId,
                role: 'assistant',
                content: rawContent,
                timestamp: new Date(),
                isError: false,
            });

            // Final scroll
            if (this.state.autoScroll) {
                this.scrollController.scrollToBottom(true);
            }
        }
    },

    // UI helpers
    showTypingIndicator() {
        if (this.elements.typingIndicator) {
            this.elements.typingIndicator.classList.add('active');
            this.elements.typingIndicator.setAttribute('aria-hidden', 'false');
        }
    },

    hideTypingIndicator() {
        if (this.elements.typingIndicator) {
            this.elements.typingIndicator.classList.remove('active');
            this.elements.typingIndicator.setAttribute('aria-hidden', 'true');
        }
    },

    showWelcomeMessage() {
        if (this.elements.welcomeMessage) {
            this.elements.welcomeMessage.style.display = 'block';
        }
    },

    hideWelcomeMessage() {
        if (this.elements.welcomeMessage) {
            this.elements.welcomeMessage.style.display = 'none';
        }
    },

    // Utility functions
    generateMessageId() {
        return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
    },

    formatTime(date) {
        return date.toLocaleTimeString('zh-CN', { 
            hour: '2-digit', 
            minute: '2-digit' 
        });
    },

    playMessageSound() {
        // Implement sound notification
        // This is a placeholder - you'd implement actual sound playback
        console.log('🔊 Message sound (placeholder)');
    },

    handleResize() {
        // Handle responsive behavior
        this.scrollController.handleResize();
    },

    handleScroll() {
        // Handle scroll events
        this.scrollController.handleScroll();
    },

    handleNearBottom() {
        // Near bottom: resume auto-follow without changing user's preference
        if (!this.scrollController?.isAutoScroll) {
            this.scrollController.setAutoScroll(true);
        }
    },

    handleAwayFromBottom() {
        // Away from bottom: pause auto-follow without changing user's preference
        if (this.scrollController?.isAutoScroll) {
            this.scrollController.setAutoScroll(false);
        }
    },

    // Session management
    async createSession() {
        try {
            const response = await fetch('/api/sessions', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    metadata: {
                        user_agent: navigator.userAgent,
                        language: navigator.language,
                        think_mode: this.state.thinkMode,
                    }
                })
            });
            
            const data = await response.json();
            return data.session_id;
        } catch (error) {
            console.error('❌ Failed to create session:', error);
            // Return a fallback session ID
            return `fallback-${Date.now()}`;
        }
    },

    // Conversation management
    async loadConversationHistory() {
        // Load conversation history from localStorage or API
        // Implementation depends on your requirements
    },

    async loadCurrentConversation() {
        if (!this.state.currentSessionId) return;
        
        try {
            const response = await fetch(`/api/chat/history/${this.state.currentSessionId}`);
            const data = await response.json();
            
            // Clear current messages
            if (this.elements.messagesContainer) {
                this.elements.messagesContainer.innerHTML = '';
            }
            
            // Load messages
            data.messages.forEach(msg => {
                this.addMessage(msg.content, msg.role);
            });
            
            this.hideWelcomeMessage();
            
        } catch (error) {
            console.warn('⚠️ Failed to load conversation:', error);
        }
    },

    startNewChat() {
        // Clear current session
        this.state.currentSessionId = null;
        this.state.messageHistory = [];
        
        // Clear messages
        if (this.elements.messagesContainer) {
            this.elements.messagesContainer.innerHTML = '';
        }
        
        // Show welcome message
        this.showWelcomeMessage();
        
        // Save state
        this.saveState();
    },

    exportConversation() {
        // Export current conversation
        // Implementation depends on your requirements
        console.log('📤 Export conversation (placeholder)');
    },

    // Markdown Renderer Class
    MarkdownRenderer: class {
        constructor() {
            this.marked = null;
            this.prism = null;
            this.katex = null;
            this.mermaid = null;
            this.streaming = false; // when true, skip heavy Prism work during streaming
        }

        setStreaming(flag) {
            this.streaming = !!flag;
        }

        async init() {
            // Load marked.js
            await this.loadScript('https://cdn.jsdelivr.net/npm/marked/marked.min.js');
            this.marked = window.marked;
            
            // Configure marked
            this.configureMarked();
            
            // Load Prism.js for syntax highlighting
            await this.loadPrism();
            
            // Load KaTeX for math rendering (optional)
            // await this.loadKaTeX();
            
            // Load Mermaid for diagrams (optional)
            // await this.loadMermaid();
        }

        configureMarked() {
            this.marked.setOptions({
                highlight: (code, lang) => {
                    // In streaming mode, skip highlighting for performance and to avoid flicker
                    if (this.streaming) {
                        return this.escapeHtml(code);
                    }
                    if (this.prism && this.prism.languages[lang]) {
                        try {
                            return this.prism.highlight(code, this.prism.languages[lang], lang);
                        } catch (err) {
                            console.warn('Prism highlighting failed:', err);
                        }
                    }
                    return this.escapeHtml(code);
                },
                breaks: true,
                gfm: true,
                tables: true,
                sanitize: false, // We handle sanitization separately
                smartLists: true,
                smartypants: true,
                langPrefix: 'language-',
            });

            // Custom renderer
            const renderer = new this.marked.Renderer();
            
            // Custom code block rendering with copy button
            renderer.code = (code, language) => {
                const validLang = language && this.prism?.languages[language] ? language : 'plaintext';
                const highlighted = (this.streaming || !this.prism) ?
                    this.escapeHtml(code) :
                    this.prism.highlight(code, this.prism.languages[validLang], validLang);

                return `
                    <div class="code-block-wrapper">
                        <div class="code-block-header">
                            <span class="code-language">${validLang}</span>
                            <button class="copy-code-btn" onclick="navigator.clipboard.writeText(\`${this.escapeHtml(code)}\`).then(() => this.showCopyFeedback(this))" title="Copy code">
                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                </svg>
                            </button>
                        </div>
                        <pre class="code-block"><code class="language-${validLang}">${highlighted}</code></pre>
                    </div>
                `;
            };

            // Custom table rendering
            renderer.table = (header, body) => {
                return `
                    <div class="table-wrapper">
                        <table class="markdown-table">
                            <thead>${header}</thead>
                            <tbody>${body}</tbody>
                        </table>
                    </div>
                `;
            };

            // Custom link rendering
            renderer.link = (href, title, text) => {
                const isExternal = href.startsWith('http') && !href.includes(window.location.hostname);
                const target = isExternal ? 'target="_blank" rel="noopener noreferrer"' : '';
                const titleAttr = title ? `title="${title}"` : '';
                return `<a href="${href}" ${target} ${titleAttr}>${text}</a>`;
            };

            this.marked.use({ renderer });
        }

        async loadPrism() {
            // Load Prism CSS
            await this.loadStylesheet('https://cdn.jsdelivr.net/npm/prismjs@1.29.0/themes/prism-tomorrow.min.css');
            
            // Load Prism JS
            await this.loadScript('https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-core.min.js');
            await this.loadScript('https://cdn.jsdelivr.net/npm/prismjs@1.29.0/plugins/autoloader/prism-autoloader.min.js');

            this.prism = window.Prism;
            if (this.prism && this.prism.plugins && this.prism.plugins.autoloader) {
                this.prism.plugins.autoloader.languages_path = 'https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/';
            }
        }

        render(markdown) {
            if (!this.marked) {
                console.warn('Marked.js not loaded, returning plain text');
                return this.escapeHtml(markdown);
            }
            
            try {
                const html = this.marked.parse(markdown);
                return this.sanitizeHtml(html);
            } catch (error) {
                console.error('Markdown rendering failed:', error);
                return this.escapeHtml(markdown);
            }
        }

        sanitizeHtml(html) {
            // Basic HTML sanitization
            // In production, use a proper sanitization library like DOMPurify
            const div = document.createElement('div');
            div.innerHTML = html;
            
            // Remove potentially dangerous elements
            const dangerous = div.querySelectorAll('script, style, iframe, object, embed');
            dangerous.forEach(el => el.remove());
            
            return div.innerHTML;
        }

        escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        async loadScript(src) {
            return new Promise((resolve, reject) => {
                const script = document.createElement('script');
                script.src = src;
                script.onload = resolve;
                script.onerror = reject;
                document.head.appendChild(script);
            });
        }

        async loadStylesheet(href) {
            return new Promise((resolve, reject) => {
                const link = document.createElement('link');
                link.rel = 'stylesheet';
                link.href = href;
                link.onload = resolve;
                link.onerror = reject;
                document.head.appendChild(link);
            });
        }

        showCopyFeedback(button) {
            // Show copy feedback
            const originalHTML = button.innerHTML;
            button.innerHTML = '✓';
            button.classList.add('copied');
            
            setTimeout(() => {
                button.innerHTML = originalHTML;
                button.classList.remove('copied');
            }, 2000);
        }
    },

    // Scroll Controller Class
    ScrollController: class {
        constructor(container, options = {}) {
            this.container = container;
            this.options = {
                threshold: options.threshold || 100,
                animationDuration: options.animationDuration || 200,
                ...options
            };
            
            this.isAutoScroll = true;
            this.isUserScrolling = false;
            this.scrollTimeout = null;
            
            this.setupEventListeners();
        }

        setupEventListeners() {
            if (!this.container) return;
            
            this.container.addEventListener('scroll', () => this.handleScroll());
            this.container.addEventListener('wheel', () => this.handleWheel());
        }

        handleScroll() {
            this.isUserScrolling = true;
            
            clearTimeout(this.scrollTimeout);
            this.scrollTimeout = setTimeout(() => {
                this.isUserScrolling = false;
                this.checkScrollPosition();
            }, 150);
        }

        handleWheel() {
            this.isUserScrolling = true;
        }

        checkScrollPosition() {
            if (!this.container) return;
            
            const { scrollTop, scrollHeight, clientHeight } = this.container;
            const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
            
            if (distanceFromBottom < this.options.threshold) {
                this.isAutoScroll = true;
                this.onNearBottom();
            } else {
                this.isAutoScroll = false;
                this.onAwayFromBottom();
            }
        }

        scrollToBottom(immediate = false) {
            if (!this.container || !this.isAutoScroll) return;
            
            const scrollHeight = this.container.scrollHeight;
            const targetScrollTop = scrollHeight - this.container.clientHeight;
            
            if (immediate) {
                this.container.scrollTop = targetScrollTop;
            } else {
                this.smoothScrollTo(targetScrollTop);
            }
        }

        smoothScrollTo(targetScrollTop) {
            if (!this.container) return;
            
            const startScrollTop = this.container.scrollTop;
            const distance = targetScrollTop - startScrollTop;
            const startTime = performance.now();
            
            const animateScroll = (currentTime) => {
                const elapsed = currentTime - startTime;
                const progress = Math.min(elapsed / this.options.animationDuration, 1);
                
                // Easing function (ease-out)
                const easeOut = 1 - Math.pow(1 - progress, 3);
                
                this.container.scrollTop = startScrollTop + (distance * easeOut);
                
                if (progress < 1) {
                    requestAnimationFrame(animateScroll);
                }
            };
            
            requestAnimationFrame(animateScroll);
        }

        setAutoScroll(enabled) {
            this.isAutoScroll = enabled;
            if (enabled) {
                this.scrollToBottom();
            }
        }

        handleResize() {
            // Handle container resize
            this.checkScrollPosition();
        }

        onNearBottom() {
            // Override in instance
        }

        onAwayFromBottom() {
            // Override in instance
        }

        on(event, callback) {
            if (event === 'scroll') {
                this.onScroll = callback;
            } else if (event === 'nearBottom') {
                this.onNearBottom = callback;
            } else if (event === 'awayFromBottom') {
                this.onAwayFromBottom = callback;
            }
        }
    },

    // Connection Manager Class
    ConnectionManager: class {
        constructor(options = {}) {
            this.options = {
                checkInterval: options.checkInterval || 30000,
                maxReconnectAttempts: options.maxReconnectAttempts || 5,
                reconnectDelay: options.reconnectDelay || 1000,
                ...options
            };
            
            this.isConnected = false;
            this.reconnectAttempts = 0;
            this.checkInterval = null;
            this.eventListeners = {};
        }

        start() {
            this.stop();
            this.checkConnection();
            this.checkInterval = setInterval(() => this.checkConnection(), this.options.checkInterval);
        }

        stop() {
            if (this.checkInterval) {
                clearInterval(this.checkInterval);
                this.checkInterval = null;
            }
        }

        async checkConnection() {
            try {
                const response = await fetch('/health');
                const data = await response.json();
                
                if (response.ok && data.status === 'healthy') {
                    if (!this.isConnected) {
                        this.isConnected = true;
                        this.reconnectAttempts = 0;
                        this.emit('connected');
                    }
                } else {
                    this.handleDisconnection();
                }
            } catch (error) {
                this.handleDisconnection();
            }
        }

        handleDisconnection() {
            if (this.isConnected) {
                this.isConnected = false;
                this.emit('disconnected');
            }
            
            if (this.reconnectAttempts < this.options.maxReconnectAttempts) {
                this.reconnectAttempts++;
                this.emit('reconnecting', this.reconnectAttempts);
                
                setTimeout(() => this.checkConnection(), this.options.reconnectDelay);
            }
        }

        on(event, callback) {
            if (!this.eventListeners[event]) {
                this.eventListeners[event] = [];
            }
            this.eventListeners[event].push(callback);
        }

        emit(event, ...args) {
            if (this.eventListeners[event]) {
                this.eventListeners[event].forEach(callback => callback(...args));
            }
        }
    }
};

// Initialize application when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => App.init());
} else {
    App.init();
}

// Export for global access
window.App = App;