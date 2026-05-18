# Celeste 项目当前实现说明

这份文档基于当前 `celeste` 项目的源码整理，目标是说明“现在已经做到了什么”。它不是设计预案，也不是理想状态清单，而是对目前可运行内容、代码结构和地图组织方式的实际总结。

## 项目定位

这是一个使用 `Rust + Bevy 0.15.1` 编写的 2D 平台动作原型，重点不在于完整复刻原作内容，而在于先把接近 Celeste 风格的角色操作手感、状态切换和视觉反馈做出来。

当前工程已经从单文件原型演进为模块化结构，包含：

- 可运行的游戏入口
- 玩家运动与状态机
- 基于 JSON 的关卡数据加载
- 房间切换与重生逻辑
- 简单危险物与 checkpoint
- 角色动画、头发表现、天气 shader 和冲刺拖尾特效

## 运行方式

在项目目录 `celeste/` 下执行：

```bash
cargo run
```

建议直接从该目录启动，而不是手动运行 `target` 下的可执行文件，因为项目依赖相对路径加载 `assets/` 资源。

## 当前操作方式

- `A / D` 或方向键左右：水平移动
- `W / S` 或方向键上下：方向输入
- `Space` 或 `Z`：跳跃
- `J` 或 `X`：抓墙 / 攀墙
- `K` 或 `C`：冲刺

## 已实现功能总览

### 1. 基础场景与运行时配置

- 使用 `Bevy` 默认插件构建 2D 游戏窗口
- 使用 `ImagePlugin::default_nearest()` 保持像素风采样
- 固定逻辑帧率为 `60Hz`
- 使用 `bevy_framepace` 将渲染帧率限制为 `60 FPS`
- 2D 相机采用固定垂直视口高度 `180`，便于维持稳定的像素感

对应入口可以参考 [main.rs](/d:/homework/rust/celeste/celeste/src/main.rs)。

### 2. 玩家基础移动

当前已经实现了比较完整的横版平台移动基础：

- 地面移动与空中移动区分不同的加速度和摩擦力
- 角色朝向会根据水平输入更新
- 支持可变跳跃高度
- 支持下落加速和低跳截断
- 支持 apex 半重力处理，让跳跃最高点更柔和
- 限制最大下落速度

这些逻辑主要位于 [player.rs](/d:/homework/rust/celeste/celeste/src/systems/player.rs) 和 [constants.rs](/d:/homework/rust/celeste/celeste/src/constants.rs)。

### 3. Jump Buffer 与 Coyote Time

为了让操作手感更接近 Celeste 风格，项目已经实现了：

- `jump_buffer_timer`：提前按下跳跃时缓存输入
- `jump_grace_timer`：离开地面后的短暂容错跳跃

这意味着角色不会因为极短时间差而“吞输入”，对平台动作手感很重要。

### 4. 墙体交互

目前墙体相关能力已经比较完整，包括：

- 贴墙检测
- 墙滑
- 抓墙
- 上下攀墙
- 墙跳
- 中立墙跳
- 攀墙跳
- 攀爬到平台顶部时的 top-out 过渡

此外还实现了两个很关键的细节：

- 向上移动时的 `upward corner correction`
- 横向冲刺撞角时的 `dash corner correction`

这两类修正会明显减少“明明应该能过去却被像素边角卡住”的问题。

### 5. 冲刺、滑行与 Super Jump

冲刺系统已经接入完整状态和反馈：

- 支持八方向冲刺
- 无方向输入时会按当前朝向水平冲刺
- 冲刺有持续时间与剩余次数管理
- 落地后会恢复一次冲刺
- 冲刺结束时会按倍率衰减速度，避免硬停
- 斜向下冲刺落地后会进入短暂 dash slide
- 在特定窗口内可触发 super jump
- 蹲姿下会使用 duck super jump 的不同倍率

项目里已经存在 `DashState`、`DashSlideState` 和 `FreezeFrameState`，说明冲刺不仅是速度变化，也包含状态推进和短暂冻结帧反馈。

### 6. 蹲下与碰撞箱切换

角色支持蹲下状态，并会同步切换碰撞箱高度：

- 地面按下时进入 crouch
- dash slide 期间也会保持 crouch
- 起身前会检查头顶空间是否允许恢复正常碰撞箱
- 攀墙和 top-out 状态会强制退出 crouch

这部分已经不是单纯的视觉切图，而是和碰撞逻辑绑定在一起的真实状态。

### 7. 玩家状态机

当前玩家状态机至少包含以下状态：

- `Normal`
- `Climb`
- `TopOut`
- `Dash`

状态切换不是分散在多个零散条件里，而是通过 `PlayerStateMachine` 统一推进，便于后续继续扩展。

### 8. 动画系统

当前动画已支持以下表现状态：

- `Idle`
- `Run`
- `Duck`
- `Climb`
- `ClimbLookback`

实现方式是基于 `TextureAtlas` 切帧，而不是复杂动画树。已经接入的资源包括：

- `idle_sheet.png`
- `run_sheet.png`
- `duck.png`
- `climb_sheet.png`
- `climb_lookback_sheet.png`

动画朝向、攀爬时正放/倒放、回头贴墙等细节都已经写好。

### 9. 头发系统

头发表现是这个项目里完成度比较高的一部分，分成前后两层：

- 前层 `bangs.png` 作为刘海 sprite
- 后层使用程序化链条模拟的头发段

当前头发系统具备这些特性：

- 头发根部会跟随角色朝向和动画偏移
- 多段头发会按约束长度跟随
- 会受到简化重力、风向和运动拖拽影响
- 冲刺时头发会产生额外反向拉扯
- 剩余冲刺次数会影响头发颜色

这部分逻辑主要在 [hair.rs](/d:/homework/rust/celeste/celeste/src/systems/hair.rs)。

### 10. Shader 与视觉效果

项目已经接入两类自定义 `Material2d` shader：

- `HairMaterial`
  - 用于后发段的像素化外轮廓渲染
- `WeatherMaterial`
  - 用于全屏天气覆盖层

除此之外还有：

- 冲刺拖尾粒子
- 短暂冻结帧效果
- 角色头发和刘海的分层渲染

冲刺拖尾并不是简单残影，而是按方向批量生成的一串短生命周期白色像素粒子。

## 地图与关卡系统

### 1. JSON 地图加载

项目已经支持从 `assets/maps/chapter_01.json` 读取地图数据。当前数据结构包括：

- 地图 `id`
- `start_room`
- 多个 `rooms`

每个房间目前支持：

- `bounds`
- `default_spawn`
- `spawn_points`
- `collision`
- `hazards`
- `checkpoints`
- `exits`

对应数据定义在 [level.rs](/d:/homework/rust/celeste/celeste/src/level.rs)。

### 2. 当前已有多房间样例

`chapter_01.json` 里已经包含至少 3 个房间：

- `room_00`
- `room_01`
- `room_02`

说明这个项目不再只是单屏测试场，而是已经具备“章节内多个房间”的基本结构。

### 3. 房间切换

项目已经实现：

- 进入出口区域后切换到目标房间
- 从目标房间指定 spawn point 出生
- 可选是否保留进入出口时的速度
- 切换房间时清除上一房间的关卡实体
- 切换相机中心和天气覆盖层位置

这套逻辑在 [systems/level.rs](/d:/homework/rust/celeste/celeste/src/systems/level.rs) 中已经打通。

### 4. Checkpoint 与重生

当前存在两种重置位置的方式：

- 掉出死亡高度阈值后回到当前 respawn point
- 触碰 hazard 后回到当前 respawn point

同时支持：

- 进入 checkpoint 后更新房间内 respawn point
- 房间切换后切换当前 respawn point

### 5. 场景几何生成

当前场景并不是手写死在 `main.rs` 中，而是运行时根据房间数据生成：

- 地面/墙体碰撞实体
- 危险物实体
- checkpoint 标记
- 出口区域
- 房间背景框
- dirt tile 图块装饰

其中 dirt tile 还会根据周围邻接关系自动选择图块索引，不是单纯铺一张重复纹理。

## 当前代码结构

项目源码目前大致按下面方式组织：

- [main.rs](/d:/homework/rust/celeste/celeste/src/main.rs)
  - 应用入口，注册插件、固定帧率和材质插件
- [scene.rs](/d:/homework/rust/celeste/celeste/src/scene.rs)
  - 初始化相机、玩家、头发、天气覆盖层和房间几何
- [components.rs](/d:/homework/rust/celeste/celeste/src/components.rs)
  - ECS 组件、状态、动画枚举与自定义材质定义
- [constants.rs](/d:/homework/rust/celeste/celeste/src/constants.rs)
  - 角色手感、冲刺、头发与渲染相关常量
- [level.rs](/d:/homework/rust/celeste/celeste/src/level.rs)
  - 地图 JSON 数据结构与读取逻辑
- [utils.rs](/d:/homework/rust/celeste/celeste/src/utils.rs)
  - 碰撞、插值、头发辅助和颜色转换等工具函数
- [systems/player.rs](/d:/homework/rust/celeste/celeste/src/systems/player.rs)
  - 输入、状态机、物理、碰撞与角色移动
- [systems/animation.rs](/d:/homework/rust/celeste/celeste/src/systems/animation.rs)
  - 玩家贴图切换与帧动画
- [systems/hair.rs](/d:/homework/rust/celeste/celeste/src/systems/hair.rs)
  - 头发模拟与渲染更新
- [systems/effects.rs](/d:/homework/rust/celeste/celeste/src/systems/effects.rs)
  - 冲刺拖尾粒子
- [systems/weather.rs](/d:/homework/rust/celeste/celeste/src/systems/weather.rs)
  - 天气材质时间更新
- [systems/level.rs](/d:/homework/rust/celeste/celeste/src/systems/level.rs)
  - 房间切换、checkpoint 与 hazard respawn
- [systems/mod.rs](/d:/homework/rust/celeste/celeste/src/systems/mod.rs)
  - 游戏系统总注册与调度顺序

整体上已经形成了比较清晰的模块边界，后续继续补系统时不需要再回到“大一统主文件”。

## 当前明确存在但尚未完全展开的部分

从代码现状看，下面这些能力要么还比较基础，要么还只是结构已经预留：

- `CollisionKind` 里已经有 `one_way_platform`、`camera_zone`、`effect_zone`
  - 但当前逻辑主要还是围绕普通实体碰撞和房间几何生成
- 地图里已有 `art_tag`
  - 但它目前更多是为后续美术扩展预留
- 天气 shader 已经接入
  - 但当前只是全屏覆盖层，并未和房间区域或天气类型深度联动
- 已有 checkpoint 和房间出口
  - 但还没有更完整的关卡流程、UI 或章节管理

## 现阶段更准确的项目结论

如果只用一句话总结当前进度，这个项目已经不是“Bevy 里放一个能左右移动的小人”，而是一个具备以下特征的可玩原型：

- 角色核心移动手感已成形
- 冲刺、墙体交互和若干 Celeste 风格技巧已落地
- 关卡数据已经从硬编码过渡到 JSON 房间结构
- 房间切换、checkpoint、hazard respawn 已接通
- 视觉表现已经覆盖动画、头发、天气和冲刺特效

也就是说，当前最扎实的部分是“角色控制与局部玩法循环”，而不是“大量关卡内容和完整游戏流程”。
