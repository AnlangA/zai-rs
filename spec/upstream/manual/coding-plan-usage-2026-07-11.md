> ## Documentation Index
> Fetch the complete documentation index at: https://docs.bigmodel.cn/llms.txt
> Use this file to discover all available pages before exploring further.

# 用量查询插件

> 查询 GLM Coding Plan 的配额和使用统计。

**glm-plan-usage** 插件允许您在 Claude Code 中直接查询 GLM Coding Plan 的当前配额和使用统计信息。

**GitHub 仓库**: [zai-coding-plugins](https://github.com/zai-org/zai-coding-plugins)

## 前置要求

* Node.js 18 或更高版本
* 安装 Claude Code CLI (参考 [Claude Code](/devpack/tool/claude))

## 快速开始

<Note>
  在 Claude Code 中安装插件市场以访问这些插件。
</Note>

<Tabs>
  <Tab title="方式一：手动安装">
    前置要求：已配置 Git 环境。

    **1. 安装插件市场**

    ```shell theme={null}
    claude plugin marketplace add zai-org/zai-coding-plugins
    ```

    **2. 安装插件**

    ```shell theme={null}
    claude plugin install glm-plan-usage@zai-coding-plugins
    ```
  </Tab>

  <Tab title="方式二：自动化工具">
    运行 `npx @z_ai/coding-helper` 工具直接管理和安装插件。

    `开始` -> `编码工具` -> `Claude Code` -> `插件市场`

    ```bash theme={null}
    npx @z_ai/coding-helper
    ```
  </Tab>
</Tabs>

## 使用方法

<Steps>
  <Step title="启动 Claude Code">
    进入项目目录并启动 Claude Code：

    ```bash theme={null}
    claude
    ```
  </Step>

  <Step title="查询用量">
    使用以下命令检查您的当前配额和使用情况：

    ```bash theme={null}
    /glm-plan-usage:usage-query
    ```
  </Step>
</Steps>

## 插件市场

<CardGroup cols={2}>
  <Card title="GitHub 仓库" icon="github" href="https://github.com/zai-org/zai-coding-plugins">
    查看源码、提交问题、贡献代码
  </Card>

  <Card title="插件市场" icon="book" href="https://code.claude.com/docs/en/plugins">
    查看如何构建插件
  </Card>
</CardGroup>
