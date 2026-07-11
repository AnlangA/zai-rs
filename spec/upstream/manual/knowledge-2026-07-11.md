> ## Documentation Index
> Fetch the complete documentation index at: https://docs.bigmodel.cn/llms.txt
> Use this file to discover all available pages before exploring further.

# 创建知识库

> 用于创建个人知识库，支持绑定向量化模型、设置名称、描述、背景色和图标。点击 **Try it** 按钮可快速试用。



## OpenAPI

````yaml /openapi/openapi.json post /llm-application/open/knowledge
openapi: 3.0.1
info:
  title: ZHIPU AI API
  description: ZHIPU AI 接口提供强大的 AI 能力，包括聊天对话、工具调用和视频生成。
  license:
    name: ZHIPU AI 开发者协议和政策
    url: https://chat.z.ai/legal-agreement/terms-of-service
  version: 1.0.0
  contact:
    name: Z.AI 开发者
    url: https://chat.z.ai/legal-agreement/privacy-policy
    email: user_feedback@z.ai
servers:
  - url: https://open.bigmodel.cn/api/
    description: 开放平台服务
security:
  - bearerAuth: []
tags:
  - name: 模型 API
    description: Chat API
  - name: 工具 API
    description: Web Search API
  - name: Agent API
    description: Agent API
  - name: 文件 API
    description: File API
  - name: 知识库 API
    description: Knowledge API
  - name: 实时 API
    description: Realtime API
  - name: 批处理 API
    description: Batch API
  - name: 助理 API
    description: Assistant API
  - name: 智能体 API（旧）
    description: QingLiu Agent API
paths:
  /llm-application/open/knowledge:
    post:
      tags:
        - 知识库 API
      summary: 创建知识库
      description: 用于创建个人知识库，支持绑定向量化模型、设置名称、描述、背景色和图标。点击 **Try it** 按钮可快速试用。
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/KnowledgeCreateRequest'
        required: true
      responses:
        '200':
          description: 业务处理成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/KnowledgeCreateResponse'
        default:
          description: 请求失败。
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/LlmApplicationError'
components:
  schemas:
    KnowledgeCreateRequest:
      type: object
      properties:
        embedding_id:
          type: integer
          enum:
            - 3
            - 11
            - 12
          description: |-
            知识库绑定的向量化模型ID。可选值：
            - 3: Embedding-2
            - 11: Embedding-3
            - 12: Embedding-3-pro
        embedding_model:
          type: string
          enum:
            - Embedding-2
            - Embedding-3
            - Embedding-3-pro
          description: |-
            知识库绑定的向量化模型code。可选值：
            - Embedding-2
            - Embedding-3
            - Embedding-3-pro
        contextual:
          type: integer
          enum:
            - 0
            - 1
          description: 是否开启上下文增强
        name:
          type: string
          description: 知识库名称
        description:
          type: string
          description: 知识库描述
        background:
          type: string
          description: 背景颜色，可选：blue, red, orange, purple, sky, green, yellow，默认blue
        icon:
          type: string
          description: 知识库图标，可选：question, book, seal, wrench, tag, horn, house，默认question
      required:
        - embedding_id
        - name
    KnowledgeCreateResponse:
      type: object
      properties:
        data:
          type: object
          properties:
            id:
              type: string
              description: 生成的知识库唯一id
        code:
          type: integer
          description: 响应码，200为成功
        message:
          type: string
          description: 响应信息
        timestamp:
          type: integer
          description: 响应时间戳
    LlmApplicationError:
      type: object
      properties:
        code:
          type: integer
        message:
          type: string
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      description: >-
        标准的 HTTP Bearer 认证方式，在 [API
        Keys](https://bigmodel.cn/usercenter/proj-mgmt/apikeys) 页面获取密钥。

````