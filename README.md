# Racing Car

Jogo de corrida 3D arcade escrito do zero em Rust com [Bevy 0.19](https://bevy.org)
e física do [Avian3D 0.7](https://github.com/Jondolf/avian). Todo o conteúdo é
procedural: a pista, os carros, o cenário e o som do motor são gerados em código,
sem depender de modelos externos.

O alvo é o desktop Linux (X11 e Wayland).

## Como jogar

```bash
cd racing-car
cargo run --release
```

Prepare-se para esperar na primeira vez: o `--release` usa LTO e uma única
unidade de codegen, então compilar o Bevy inteiro levou 32 minutos na máquina em
que o projeto foi escrito. Vale para gerar o binário final, não para desenvolver.

No dia a dia use o perfil `dev`, que compila em poucos minutos e roda liso —
as dependências vão com `opt-level = 3` e só o código do jogo fica em
`opt-level = 1`:

```bash
cargo run          # jogável, e recompila o jogo em ~30 s
cargo clippy       # lint
cargo fmt          # formatação
```

## Controles

| Tecla                | Ação                                                  |
| -------------------- | ----------------------------------------------------- |
| `W` / `↑`            | Acelerar                                              |
| `S` / `↓`            | Frear e, parado, engatar a ré                         |
| `A` `D` / `←` `→`    | Esterçar                                              |
| `Espaço`             | Freio de mão (trava o eixo traseiro e solta a traseira) |
| `C`                  | Alternar câmera: perseguição, cockpit e transmissão   |
| `R`                  | Voltar para a pista no ponto mais próximo             |
| `Esc`                | Abandonar a corrida e voltar ao menu                  |

A caixa é automática: a marcha sobe e desce sozinha conforme o RPM, e a ré entra
quando você insiste no freio com o carro parado.

### Ajustes e depuração

| Tecla       | Ação                                            |
| ----------- | ----------------------------------------------- |
| `1` / `2`   | Diminuir / aumentar o ângulo de obturador do motion blur |
| `3` / `4`   | Diminuir / aumentar as amostras do motion blur  |
| `F2`        | Alternar a cor do overlay de FPS                |
| `F3`        | Ligar / desligar o overlay de FPS               |
| `F4`        | Ligar / desligar o gráfico de frame time        |
| `F5` / `F6` | Diminuir / aumentar a fonte do overlay          |
| `F7`        | Gizmos de colisores do Avian                    |
| `F8`        | Gizmos do traçado da pista e dos checkpoints    |
| `F9`        | Salvar um PNG da janela em `screenshots/`       |

## O que está implementado

- **Pista por spline.** Uma `CubicCardinalSpline` cíclica é reamostrada numa
  tabela parametrizada por comprimento de arco. Essa tabela é a fonte única de
  verdade: malha, guard-rails, checkpoints, grid de largada, IA, minimapa e
  respawn leem todos dela.
- **Malha procedural.** Asfalto com UV que acompanha o arco, linhas de borda,
  zebras vermelho/branco nas curvas, acostamento de grama e linha de largada,
  tudo montado como `TriangleList` com normais suavizadas. As texturas de
  asfalto, grama e cascalho são geradas por ruído em `Image::new_fill`.
- **Carro raycast-vehicle.** O chassi é um único `RigidBody::Dynamic` com centro
  de massa rebaixado. As quatro rodas são pontos de raycast: cada uma resolve
  suspensão (mola + amortecedor), tração longitudinal e grip lateral limitado por
  um círculo de atrito. Transferência de peso, subviragem e drift saem daí,
  sem simular curva de slip de pneu.
- **Modelo 3D montado em código.** Chassi, capô, cockpit, spoiler sobre hastes,
  difusor, faróis emissivos, lanternas, escapamentos e rodas que giram e esterçam
  conforme a suspensão.
- **Adversários.** Pilotos de IA seguem a tabela da pista com steering
  proporcional ao erro lateral e velocidade-alvo calculada pela curvatura à
  frente. Cada um tem linha de corrida e ritmo levemente diferentes, e colidem de
  verdade entre si e com o jogador. Quem capota ou fica encalhado depois de um
  toque volta sozinho para o traçado depois de três segundos.
- **Corrida completa.** Grid, contagem regressiva, checkpoints como sensores do
  Avian validados em ordem, contagem de voltas, classificação ao vivo e tela de
  resultados.
- **Câmera.** Perseguição com spring-arm amortecido, cockpit na altura do capô e
  transmissão, que salta entre torres fixas na beira da pista como numa
  transmissão de TV. Todos os modos usam `MotionBlur` por objeto, ajustável em
  tempo real.
- **Interface.** Splash, menu principal com tema de corrida, telas de ajustes
  (qualidade, volume, voltas, adversários), HUD com velocímetro, marcha, barra de
  RPM, voltas, tempos e posição, além de um minimapa que rasteriza o traçado numa
  textura e desenha os carros por cima.
- **Áudio.** Música no menu e na corrida, mais um motor sintetizado via
  `Decodable` cujo pitch acompanha o RPM.
- **Persistência.** Ajustes, melhor volta e corridas concluídas ficam em
  `$XDG_DATA_HOME/racing-car/profile.ron` (por padrão
  `~/.local/share/racing-car/profile.ron`), em RON.

## Dependências

| Crate     | Versão   | Para quê                                                  |
| --------- | -------- | --------------------------------------------------------- |
| `bevy`    | 0.19.0   | Motor: ECS, render, UI, áudio e o `bevy_dev_tools` do overlay de FPS |
| `avian3d` | 0.7.0    | Física: corpos rígidos, colisores, raycast e sensores      |
| `rand`    | 0.10     | Variação dos pilotos de IA e espalhamento do cenário       |
| `serde`   | 1.0      | Serialização do perfil salvo                               |
| `ron`     | 0.12     | Formato do arquivo de perfil                               |

As features do Bevy são listadas à mão no `Cargo.toml` para deixar de fora o
`bevy_gilrs`: o suporte a gamepad exige a `libudev` instalada no sistema e o
protótipo é só de teclado.

Além do toolchain Rust (edition 2024), o build precisa apenas do que o Bevy já
pede num desktop Linux: `libasound2-dev` para áudio e as bibliotecas de X11 ou
Wayland da sua distribuição.

## Estrutura

```
src/
├── main.rs        monta a janela e registra os plugins
├── core/          estados, configurações persistidas, camadas de física
├── world/         cenário e pista (spline, malha, adereços)
├── vehicle/       modelo, física raycast, input e IA
├── camera/        modos de câmera e motion blur
├── race/          checkpoints, fluxo da corrida e recordes
├── ui/            tema, menu, HUD, minimapa e resultados
├── audio/         música e motor sintetizado
└── dev/           overlay de FPS e gizmos de depuração
```

Cada módulo expõe um `Plugin`, e o `main.rs` não tem lógica de jogo.

## Fora de escopo

Multiplayer, física de pneu com curva de slip real, sistema de dano, suporte a
volante ou gamepad e portes para fora do Linux desktop.

## Examples of commits

```
git add . && git commit -m ":rocket: Initial commit." && git push
git add . && git commit -m ":building_construction: Added initial project architecture." && git push
git add . && git commit -m ":building_construction: Update project architecture." && git push
git add . && git commit -m ":memo: Updated project documentation." && git push
git add . && git commit -m ":memo: Updated code documentation." && git push
git add . && git commit -m ":white_check_mark: Added feature xyz." && git push
git add . && git commit -m ":wrench: Fixed xyz usage." && git push
git add . && git commit -m ":heavy_minus_sign: Removed xyz." && git push
git add . && git commit -m ":memo: Adjusted project imports." && git push
git add . && git commit -m ":arrow_up: Updated dependencies." && git push
git add . && git commit -m ":arrow_down: Removed dependencies." && git push
git add . && git commit -m ":wastebasket: Removed unused code." && git push
git add . && git commit -m ":test_tube: Added test functionality xyz." && git push
git add . && git commit -m ":construction_worker: Building in progress." && git push
git add . && git commit -m ":construction_worker: Added CI build system." && git push
```

## License

MIT License

Copyright (c) 2026 William Franco

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.