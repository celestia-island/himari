<!-- markdownlint-disable MD033 MD041 -->
<!-- The logo lives in celestia-island.github.io/res/logos/himari.webp; that asset does not
exist yet, and this repository does not keep a copy of org assets. Add the <img> line here
once the logo lands, then drop this comment. -->
<h1 align="center">Himari</h1>

<p align="center"><strong>Activos de fórmulas: una sola fuente de verdad, tres superficies, compiladas a Rust.</strong></p>

<div align="center">
<!-- CI -->
<a href="https://github.com/celestia-island/himari/actions/workflows/ci.yml">
<img src="https://img.shields.io/github/actions/workflow/status/celestia-island/himari/ci.yml?label=CI" alt="CI" />
</a>
<!-- depspec -->
<a href="https://github.com/celestia-island/himari/actions/workflows/depspec-lint.yml">
<img src="https://img.shields.io/github/actions/workflow/status/celestia-island/himari/depspec-lint.yml?label=depspec" alt="depspec" />
</a>
<!-- License -->
<a href="https://github.com/celestia-island/sysl">
<img src="https://img.shields.io/badge/license-SySL%201.0-blue" alt="License: SySL 1.0" />
</a>
<!-- crates.io -->
<a href="https://crates.io/crates/himari">
<img src="https://img.shields.io/crates/v/himari.svg" alt="crates.io" />
</a>
<!-- docs.rs -->
<a href="https://docs.rs/himari">
<img src="https://docs.rs/himari/badge.svg" alt="docs.rs" />
</a>
<!-- Rust -->
<a href="https://www.rust-lang.org/">
<img src="https://img.shields.io/badge/rust-1.90-orange.svg" alt="Rust 1.90" />
</a>
</div>

<div align="center">

[简体中文](./docs/zh-Hans/README.md) ·
[繁體中文](./docs/zh-Hant/README.md) ·
[日本語](./docs/ja/README.md) ·
[한국어](./docs/ko/README.md) ·
[Français](./docs/fr/README.md) ·
**Español** ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

</div>
<!-- markdownlint-enable MD033 MD041 -->
**Activos de fórmulas: una sola fuente de verdad, tres superficies, compiladas a Rust.**

> **NOTICE: Este software incluye código generado por inteligencia artificial. Consulte el archivo LICENSE para los términos de la Synthetic Source License, incluidos los requisitos de divulgación de modelos.**
> Cada línea de código fuente de este repositorio fue generada por modelos de IA bajo dirección humana; ni una sola línea fue escrita a mano. Los modelos implicados se enumeran al final de [LICENSE.txt](LICENSE.txt).
> Cada capacidad reivindicada en este README está respaldada por pruebas de este repositorio y pasó por seis rondas de revisión adversarial independiente — pero **la revisión por IA no sustituye la revisión humana**,
> y en el momento de escribir esto el contenido matemático no ha sido revisado por un matemático.
> El nombre proviene de Himari Akayama (Millennium Science School · Veritas) de Blue Archive —
> convertir ideas en máquinas que funcionan. Ni el nombre en crates.io ni el nombre del repositorio de la organización colisionan con nada (comprobado el 2026-11-23).

## Qué es esto

- Usted escribe **una** fórmula (subconjunto de LaTeX / notación de artículo / estilo MATLAB, a su elección); se baja al mismo
  **IR dimensionado**, y luego se compila en una función Rust con intermedios con nombre y comentarios de fuente.
- Las dimensiones (las siete magnitudes base kg·m·s·A·K·mol·cd) se imponen en tiempo de compilación: `m + s` simplemente no compila.
- La diferenciación simbólica, la eliminación de subexpresiones comunes (CSE) y el plegado de constantes vienen integrados; las derivadas parciales analíticas están fijadas por pruebas contra diferencias centrales.
- **Las matrices son de primera clase**: las matrices concretas se verifican por completo; las **matrices de patrón** (con elipses) expresan tamaños dinámicos —
  el `[b c ⋯ 0; a b c ⋯; ⋯ a b c; 0 ⋯ a b]` de los manuales es código fuente tal como se escribe.

## Federación y escritura multilínea (segunda decisión del 2026-11-23)

**Un solo bloque de macro puede contener todo un grupo de fórmulas que se referencian entre sí** («federación»):

```rust
formula_matlab!{
    decl { F : force [N]; m : mass [kg]; r : radius [m]; }
    accel = F / m
    mech = ( accel * r )^(1/2)
}
// expands to a let chain: let accel = F / m; let mech = (accel * r).sqrt(); mech
```

- **Separador de elementos** = punto y coma o **salto de línea**, y esto **se cumple en los tres carriles** (el carril de tokens reconstruye
  los saltos de línea a partir de los números de línea de los spans de proc-macro, y esos números están alineados con el archivo fuente, así que los diagnósticos apuntan a la misma línea).
- **Los elementos posteriores pueden referenciar a los anteriores**: las dimensiones se propagan con el nombre (los m/s² de accel se unen a la tabla de símbolos); el carril build.rs inserta la sustitución en el momento de la generación.
- **El nombre es el contrato**: cuando una fórmula reutiliza un nombre de `decl`, la dimensión de su resultado debe coincidir con esa declaración
  (`a = F/m` está bien, `s : Pa; s = F/d` se rechaza); los nombres de fórmula duplicados dentro de un mismo conjunto de fórmulas o de un mismo directorio de generación siempre se rechazan.
- **Modelo de continuación de operadores**: los operadores deben ir al **final de la línea** (`x +⏎ y` ✓); un `-`/`+` inicial en la línea siguiente comienza el signo de un nuevo elemento, no es una resta continuada en la línea — esa es la decisión de desambiguación.
- La multiplicación implícita (yuxtaposición) en la superficie LaTeX **debe permanecer en una sola línea**: yuxtaponer a través de líneas se traga la fórmula siguiente como término multiplicativo.

**Escritura de matrices en varias líneas** (una línea física por fila; el espaciado de columnas es libre, no se requiere alineación):

```text
[ 2*k, -k,    ...,   0 ;
  -k,  2*k,   -k,    ... ;
  ..., -k,    2*k,   -k ;
  0,   ...,   -k,    2*k ]
```

- Separador de filas = `;` o salto de línea (combinables, opcionalmente al final); separador de columnas = coma o espacio (≥1, cualquier número).
- Una elipsis `...` / `⋯` / `…` en una celda significa «omitido en esa posición» (se infiere si es horizontal/vertical/diagonal al reescribir).
- **Un `-` escrito sin espacio delante es un signo, un espacio precedente inicia una nueva columna** (semántica de MATLAB): `[k-1]` es una columna, `[k -1]` son dos.

## Números complejos: canalizados de extremo a extremo desde la capa de valores hasta la generación de código (2026-11-23)

- **`j` es la unidad imaginaria reservada** (siempre adimensional, nunca declarada, **nunca en la lista de parámetros**). `Z = R_s + j*X` se escribe directamente.
- La capa de valores tiene exactamente dos variantes: `Value::Scalar(C)` / `Value::Matrix(Vec<Vec<C>>)`, con `C = {re, im}`.
  Un número real es un complejo con parte imaginaria nula ⇒ **las matrices complejas, la transposición, la multiplicación, la suma y la resta son
  verdaderas elemento a elemento gratis**; no hace falta dividir en cuadrantes «matriz real / matriz compleja».
- Las funciones trascendentes comparten una sola tabla para real y complejo: `exp ln sqrt sin cos tan sinh cosh tanh` +
  `abs arg conj re im` (las cuatro últimas siempre aterrizan de vuelta en el eje real). Las potencias complejas pasan por `exp(w·ln z)` en la rama principal.
- **`eval` informa un error explícito en valores complejos** (no descarta en silencio la parte imaginaria); use `eval_value` siempre que quiera valores complejos.
- **La generación de código tiene un carril complejo separado**: cuando una fórmula contiene `j` se emite como `-> (f64, f64)` = (real, imaginario),
  y de los auxiliares locales al bloque (`__c_mul` / `__c_sqrt` …) **solo se emiten los que realmente se usan** (de lo contrario clippy `-D warnings` se pone en rojo).
  La entrada por lotes es `out: &mut [(f64, f64)]`, directamente utilizable para barridos de frecuencia EIS.
  El carril de macros (`formula!` etc.) clasifica igual: `j` **no** se degrada a variable libre en el punto de llamada;
  un enlace ya complejo dentro de una cadena `let` federada se hereda correctamente (`emit_block_complex_with`).
  El carril complejo **no genera derivadas analíticas**: `abs/arg/conj/re/im` no son analíticas complejas, así que la regla de la cadena no se cumple.
- **El código generado y la capa de valores deben coincidir punto por punto** es una restricción estricta: `__c_powf` copia la clasificación entera de `C::pow_real`
  (una potencia entera de una base real negativa debe aterrizar exactamente en el eje real), y `__c_sqrt` usa `hypot` como la capa de valores para evitar desbordamiento en magnitudes grandes;
  que `__c_div` siga IEEE mientras la capa de valores da error es deliberado — cada uno de estos está protegido por una prueba de comparación por operador en las fixtures.
- Las cadenas de unidades aceptan exponentes racionales: `[V/A*s^(-1/2)]` (los corchetes son obligatorios — si no, el `/` del exponente no se puede distinguir del
  signo de división de la cadena de unidades); un exponente que no divide de forma exacta (`m^(1/3)`) se rechaza explícitamente. Escriba `[1]` para una magnitud adimensional
  (ganancia / conteo / fracción molar deben poder declararse todos; no pueden colarse «omitiendo la unidad»).
- **`himari-fmt --diff <file>`**: vista previa de solo lectura que imprime un diff unificado `--- current / +++ canonical`
  (sin cambios → sin salida y código de salida 0; con cambios → código de salida 1).
- **La frontera semántica entre real y complejo** (deliberada, no un descuido): `1/0` solía dar `inf` y ahora **da error**;
  `ln(0)` igualmente; `(-2)^0.5` informa «el resultado es complejo, use `eval_value`».
  El código generado sigue la semántica IEEE en su lugar (la división por cero da inf/NaN): la ruta caliente no lleva ruta de error,
  y todo lo que la capa de fórmulas puede probar estáticamente como erróneo lo atrapan las puertas de dimensión y forma.
- **`python3 scripts/fmt_check.py --fix`**: un solo comando para rehacer la maquetación de la forma canónica y regenerar los archivos auxiliares.

## Unidades: prefijos SI, conversión, grupos adimensionales (2026-11-23)

- **Busque el nombre completo antes de dividir un prefijo**: `m` es el metro (no «mili-nada»), `mol` es el mol, `cd` es la candela,
  `daN` es el decanewton (gana el prefijo más largo). `nF` / `GHz` / `mm^2` / `mV/mm` / `µF` / `Ω` funcionan todos.
  Las unidades derivadas cubren las seis que la electrónica no puede evitar: `F` `Ω` (o `ohm`) `S` `H` `Wb` `T`.
- **La dimensión y el factor de conversión se almacenan por separado**: `parse_unit` da solo la dimensión; `parse_unit_spec` da además el factor de conversión →SI;
  `convert(v, "nF", "F") = 1e-9 F`, y una discrepancia de dimensión da error. La unidad base de masa es el **kg**:
  `g = 1e-3 kg`, `mg = 1e-6 kg`, `Mg = 1000 kg`; los prefijos cuentan desde el gramo (fijado por una prueba de regresión).
  **Los parámetros de las funciones generadas son siempre unidades base SI**, un prefijo en la declaración solo afecta a la legibilidad — el comentario de doc del código generado
  explicita el factor de conversión por parámetro (citado textualmente de un comentario de doc generado real:
  `⚠ 参数 C_dl 声明为 µF：入参是 SI 基单位（1 µF = 0.000001 …）`).
- **Los prefijos CGPM 2022** están en la tabla: `Q` (1e30), `R` (1e27), `r` (1e-27), `q` (1e-30), junto a los
  `da … Y / d … y` existentes.
- **Unidades no SI con definición exacta** (`unit_base` lleva cada factor para que `convert` siga siendo dimensionalmente honesto): `min`, `h`, `d`, `L`
  (o `l`; `mL` se descompone como un prefijo sobre `L`), `deg`, `arcmin`, `arcsec`, `bar`, `atm`, `eV`, `Wh`, `tonne`, `Da` (o `u`), `nmi`, `knot`, `ha`, `acre`.
- **Las unidades con desplazamiento / logarítmicas se rechazan explícitamente**: `degC` (desplazamiento) y `dBm` (una razón) se detienen antes de dividir el prefijo,
  informando la «causa verdadera» en lugar de «unidad desconocida» — tratarlas en silencio como K o como adimensionales calcula un **valor incorrecto**.
  **Los símbolos ambiguos se rechazan igual, sin adivinar**: `cal` / `Cal` difieren entre la caloría termoquímica (4.184 J) y la caloría IT (4.1868 J), así que ambas se
  rechazan con la indicación de escribir `J`; `rpm` necesita un eje angular (1 rpm = 2π/60 rad/s) mientras que esta instalación trata `rad` como adimensional, así que se rechaza
  con la indicación de escribir `2 * pi * f` con una declaración `[Hz]`.
- **Grupos adimensionales (Buckingham Π)**: `dimensionless_groups` elimina con **racionales exactos** (`f64` deriva),
  y da las combinaciones adimensionales de una base directamente de «nombres de variables + dimensiones»; la cuenta = (número de variables) − rango (el teorema Π como aserción).
  La comprobación clásica: `{rho, v, L, mu}` → 1 grupo `rho·v·L·mu^-1` (el número de Reynolds).
  `dim_solve` es el uso inhomogéneo del mismo sistema: `{kg, m/s²} → N` da los exponentes `[1, 1]`,
  y una dimensión que no se puede formar informa «no se puede formar» más cuántas dimensiones faltan.

## Dimensiones: exponentes semienteros (2026-11-23)

`Dim` almacena internamente en unidades de **1/2** (`[i16;7]` guarda el doble del exponente real), así que las potencias fraccionarias y enteras
pasan por la misma aritmética entera: `√s = s^(1/2)`, `f^(-1/2)`, `Ω·s^(-1/2)` son todas legales, y la visualización usa la forma entre paréntesis
como `s^(-7/2)`. La impedancia de Warburg vuelve por tanto a la forma de los manuales `Z = σ·f^(-1/2)`,
sin necesidad de adimensionalización por frecuencia de referencia. El exponente de potencia de `dim_check` **pliega primero las constantes**,
así que la escritura natural `f ^ (1/2)` pasa la puerta sin llamar antes a `simplify`.

## Familias de funciones: extremo / clamp / operadores matriciales (2026-11-23)

La **única fuente de verdad** para la aridad de funciones es `func_arity` (en core); los cuatro carriles (comprobación de aridad / evaluación / dimensión / emisión / reescritura)
la consultan — antes cada carril llevaba su propio `match`, y así fue como «`arg` emitido como un método inexistente en el carril real» sobrevivió a una versión.

| familia | funciones | notas |
| --- | --- | --- |
| escalar unaria | `exp ln log sqrt sin cos tan sinh cosh tanh` | la misma tabla para real y complejo |
| compleja | `abs arg conj re im` | `arg` es adimensional; en el carril real re/im/conj son la identidad |
| extremo/clamp | `min(a,b)` `max(a,b)` `clamp(v,lo,hi)` | misma forma que Rust: `v.max(lo).min(hi)`; los tres argumentos deben compartir una dimensión |
| operadores matriciales | `det` `trace` `adjoint` `inv` | **solo aceptan literales matriciales concretos**; `det`/`inv` hasta orden 4/3 (suficiente para elementos isoparamétricos 2D y 3D), `adjoint` = traspuesta conjugada |

`det`/`inv` usan **recursión por cofactores** (`det` ≤ 4, `inv` ≤ 3): suficiente para los mapas isoparamétricos de triángulos 2D y tetraedros 3D,
con comprobaciones numéricas para `J·J⁻¹ = I` y `det(J)=1`; una inversa 4×4 necesitaría 16 cofactores 3×3
(≈100 términos) y haría explotar el código generado, así que se rechaza explícitamente con la indicación de usar una biblioteca numérica. `det`/`trace` **se expanden antes de diferenciar** en `diff` —
la expansión es una expresión diferenciable ordinaria, así que el `area = det(J)/2` de FEM tiene a la vez valor y derivada analítica
(`∂area/∂x₂ = (y₃-y₁)/2`).
La selección por tramos en `min/max/clamp` no es expresable en el IR (no hay operador de comparación), así que **no se generan derivadas**;
una línea de comentario en el código generado explica por qué (ya no entra en pánico). Extremo y clamp siguen la regla de NaN de `f64::min/max`
(pueden devolver el otro operando); los llamadores que requieran que NaN se propague deben comprobar `is_finite` en la frontera.

## Formas integradas de un vistazo

`identity` / `diag(d)` / `diag_v(d[i])` / `tridiag(a,b,c)` / `tridiag_v(b[i])` /
`bidiag_u(b,c)` / `bidiag_l(a,b)` / **`blockdiag2(p,q)` = diag(p·I₂, q·I₂)** /
**`banded(v, k)` = matriz banda simétrica con semi-ancho de banda k** (k=2 solo es inequívoca a partir de 5×5; en 4×4 degenera en una matriz densa).

Estos nombres describen la **forma de IR reconocida**; no son funciones que se puedan llamar desde una fórmula. El punto de entrada es el
boceto de matriz con elipses, y todo lo que no se reconoce se rechaza explícitamente.

Semántica de bloque diagonal: dentro de un bloque = escalar × identidad (p en la diagonal, fuera de la diagonal dentro del bloque vacío), fuera de los bloques debe estar vacío
(`0` y `...` son equivalentes); con una dimensión total impar, la celda central va al bloque inferior derecho.

## Rendimiento del carril complejo: el lado honesto (medido el 2026-11-23)

`bin/bench.rs` ahora mide cuatro carriles. El complejo, Z_c (barrido de frecuencia EIS), **es más lento que el escrito a mano**; registrado tal cual:

| forma | ns/punto | respecto al escrito a mano |
| --- | --- | --- |
| especialización escrita a mano (`σ/√ω·e^{-jπ/4}`, sin ramas, vectorizable) | 2.3 | 1× |
| transcripción de la misma forma (la secuencia de operadores del código generado copiada en un solo bucle) | 2.9 | 1.3× |
| código generado · llamada directa escalar | 6.4 | 2.8× |
| código generado · entrada por lotes | 12.8 | 5.5× |

Desglose: **el √/÷ complejo genérico cuesta unas 2,2 veces más que la especialización escrita a mano** (1 sqrt frente a 3 sqrt + una división),
y **el envoltorio por lotes cuesta otras 2,5 veces más** (escrituras en porciones de tuplas + bifurcaciones impiden que el bucle se vectorice).

Optimizaciones ya realizadas (cada una estrechó realmente la brecha):

1. `__c_sqrt` pasó de forma polar (atan2+cos+sin, 4 funciones trascendentes) a una **forma cerrada algebraica**;
1. ruta rápida puramente imaginaria `√(iy) = √(|y|/2)(1+sign·j)` (solo 1 sqrt — la forma cotidiana en EIS);
1. `__c_div` usa la forma ingenua en magnitudes ordinarias (1 división);
1. los auxiliares complejos pasaron de **cierres** a `#[inline(always)] fn` locales al bloque (15.5 → 11.9 ns/punto).

**Conclusión (escrita en el README en lugar de ocultarse)**: el carril complejo sirve para «calcula correctamente + suficientemente rápido», y no merece la pena
perseguir el límite de una especialización escrita a mano; para el bucle realmente caliente de un barrido EIS, escriba la especialización a mano y mantenga la fórmula en el
archivo `.formula` como **fuente de verdad y comprobación de dimensión**. El carril real no tiene este problema (escalar 1,11×, a la par del escrito a mano).

## Entradas por lotes: la longitud fija gana, las porciones pierden (medido)

El generador emite **dos** entradas por lotes por fórmula:

| forma | firma | medido (BV, 100k puntos) |
| --- | --- | --- |
| versión porción | `bv__batch(p0: &[f64], …, out: &mut [f64])` | 24.1 ns/punto = **0,82× del escalar (22 % más lento)** |
| versión longitud fija | `bv__batch_n<const N: usize>(p: &[f64; N], …, out: &mut [f64; N])` | 19.3 ns/punto = **1,03× del escalar (3 % más rápido)** |

Conclusión: **si quiere rendimiento, ponga la longitud en un parámetro de tipo**. Las ocho comprobaciones de límites por punto de la versión porción están dentro del bucle y
LLVM no puede eliminarlas todas; calcular la longitud común una vez (una cadena `.min()`) o pasar a iteradores zip solo llega a 0,83×.
La versión porción se mantiene como forma cómoda (datos de longitud variable); use la versión de longitud fija en rutas calientes.
(fuente del bench: `fixtures/three/src/bin/bench.rs`; las dos trampas de metodología que encontramos — construir vectores dentro de la región cronometrada, y que
vectores constantes permitan plegar el camino escalar — están registradas en los comentarios del código fuente.)

## Álgebra matricial (forma + evaluación + generación)

Las dimensiones y las formas son **dos ejes de comprobación ortogonales**: las dimensiones gobiernan el significado físico, las formas la legalidad algebraica.

| capacidad | comportamiento |
| --- | --- |
| `shape_of` | matriz concreta → `(rows, cols)`; la transposición intercambia; matriz simbólica/de patrón → `None` (sin inferencia dinámica) |
| `shape_check` | la suma/resta de matrices exige formas iguales, el producto matricial exige `left cols = right rows`, las matrices irregulares se rechazan, la división matricial se rechaza explícitamente |
| `eval_value` | evaluación unificada de escalares/matrices, producto matricial `C_ij = Σ_k A_ik·B_kj` |
| `flatten_matrix_value` | pliega una expresión matricial **estáticamente expandible** en un IR de matriz concreta (transposición/suma/resta/negación/difusión escalar/producto matricial) |
| emisión | una matriz plegada → un literal constante `vec![vec![…], …]`; las matrices grandes pasan por el carril de matriz de patrón + bucle |

```text
[1, 2; 3, 4] * [5, 6; 7, 8]  →  [[19.0, 22.0], [43.0, 50.0]]
[1, 2, 3]'                   →  3×1 column vector (emitted as vec![vec![1.0], vec![2.0], vec![3.0]])
A * [1, 2]                   →  scalar broadcast (a bare symbol is a scalar: matrices only come from literals/pattern generators)
[1, 2, 3] * [1, 2, 3]        →  compile-time rejection: matrix multiply impossible: (1, 3) × (1, 3)
```

> **Un `A,b` genérico en tiempo de ejecución sigue teniendo que declararse como tal**: los nombres de `decl` con una magnitud que termina en `_matrix`/`_vector` (o igual a `matrix`/`vector`)
> se convierten en parámetros matriz/vector, que es lo que usa el carril en tiempo de ejecución de abajo. Cualquier *otro* símbolo desnudo es un escalar, así que `A * x` sobre
> matrices cuyas formas no forman parte de la declaración aún se expresa con literales, expresiones estáticamente expandibles o generadores de patrón.
> Para un `Ax=b` general a gran escala, inferencia de tamaño en tiempo de ejecución, o sistemas mal condicionados, use una biblioteca de álgebra lineal numérica.

**Potencias enteras no negativas de matrices**: `[a, b; c, d] ^ 2` lo pliega el generador en multiplicaciones matriciales explícitas (0–8 veces),
`M ^ 0` es la matriz identidad del mismo orden (el tipo de retorno sigue siendo una matriz, no el escalar `1`). La comprobación numérica de extremo a extremo está en
`matrix_power.formula`. Un plegado fallido (potencia negativa/fraccionaria, fuera de rango, no cuadrada) emite `compile_error!`
en lugar de un accidente de compilación como `vec![…] * vec![…]`; la capa de valores (`himari-eval`) da el mismo resultado para la misma forma.

**Dos implementaciones independientes deben coincidir**: la generación de código de expresiones matriciales (`flatten_matrix_value` + emisión de literales)
y la evaluación por la capa de valores (`eval_value`) están escritas por separado, así que una **prueba diferencial** mantiene ambos lados alineados —
evaluar el literal plegado debe ser igual a evaluar la expresión no plegada celda a celda (`mat-expr-fold`).

**Un boceto de matriz sin elipses es una matriz concreta**: pasa por el carril de literales (las constantes se pliegan, sin bucle de asignación),
que supera al generador; para usar el carril del generador (donde `n` es un parámetro en tiempo de ejecución) hay que escribir las elipses.

## El carril matricial/vectorial en tiempo de ejecución (2026-11-23)

Todo lo anterior es estático: las formas se conocen cuando la fórmula compila. El carril en tiempo de ejecución es la respuesta separada para
**parámetros cuyos tamaños solo existen en tiempo de ejecución** — un solucionador que consume datos del usuario, un ensamblaje FEM, cualquier cosa que reciba `A` y `b` como valores.

- **La declaración es el único interruptor**: un `decl` cuya magnitud termina en `_matrix`/`_vector` (o es igual a `matrix`/`vector`) se convierte en un
  parámetro matriz/vector — `A : stiffness_matrix [N/m]` llega como `&[Vec<f64>]`, `b : load_vector [N]` como `&[f64]`.
  Los nombres que nunca se adhieren siguen siendo escalares, así que las fórmulas existentes no se ven afectadas (`Decl::runtime_kind`).
- **Las funciones generadas pueden fallar**: devuelven `Result<_, himari::runtime::NumericError>`, un único tipo de error estructurado
  (`InvalidInput` / `ShapeError` / `NonFiniteValue` / `SingularMatrix` / `IllConditioned` / `UnstableResidual` / `NoRealRoots`). No hay
  **ningún contrato de NaN silencioso** en este carril.
- **`solve(A,b)`** es LU con pivoteo parcial, y luego dos puertas independientes antes de que se devuelva cualquier respuesta:
  una **estimación de rcond** (`1/(‖A‖∞·‖A⁻¹‖∞)`, exigiendo `rcond ≥ f64::EPSILON·64·n`) y una **comprobación del residuo relativo**
  (`≤ 1e-9`). La inversa se forma solo para la estimación del condicionamiento, nunca como forma de obtener `x`. Un pivote singular, una
  matriz mal condicionada y una solución con residuo grande son tres errores distintos, no uno solo «fallido».
- **También soportados en este carril**: `inv`, `det` (solo argumentos literales — un `det` en tiempo de ejecución se rechaza porque su dimensión necesita un orden de matriz
  estático), `A*x`, `dot`, `norm`, y la indexación con `A(i, j)` usando índices **basados en 1** (`[I,J]`; fuera de rango y cero son errores, no pánicos).
- **Qué emite**: el cuerpo es una expresión comprobada sobre cadenas `let` en ámbito dentro de la función generada, y el tipo de retorno sigue el
  tipo de valor (`Result<f64, _>`, `Result<Vec<f64>, _>`, `Result<Vec<Vec<f64>>, _>`). Las fórmulas del carril en tiempo de ejecución no emiten **ningún** jacobiano analítico, **ninguna**
  variante `__f32` y **ninguna** entrada por lotes (esas necesitan una firma escalar de tamaño fijo).
- **Las respuestas se comprueban contra referencias independientes**: `runtime_matrix.formula` verifica `solve(A,b)` contra una eliminación gaussiana con pivoteo
  separada, `A*x` contra `b`, `A·inv(A)` contra las columnas de la identidad, la indexación basada en 1, y un sistema de epsilon deliberadamente diminuto
  (`[[1e-17, 1], [1, 1]]`) que solo el pivoteo resuelve bien — todo con tolerancias de 2e-12/2e-15.
- **Carril de macros**: `formula_matlab!` también acepta fórmulas en tiempo de ejecución, pero solo **una fórmula por bloque** (una cadena en tiempo de ejecución se queda en el carril build.rs
  con un mensaje explícito); los parámetros matriciales complejos en tiempo de ejecución se rechazan por ahora.

## Calidad de la generación de código (2026-11-23)

- **Las potencias enteras emiten `powi`**: `x^3` → `(x).powi(3)` (antes emitía un método interno inexistente,
  de modo que cualquier `x^3` no compilaba con el error apuntando dentro del código generado). La fixture tiene una **puerta de compilación** para `x^3`.
- **Las funciones desconocidas fallan en tiempo de compilación**: las funciones del carril real son una lista blanca; `arg(x)` no tiene sentido sobre los reales →
  `compile_error!`; `re/im/conj` son la identidad sobre los reales (exacta, no aproximada).
  Las llamadas con varios argumentos se rechazan en el análisis (el emisor solo miraba `args[0]`, y descartar en silencio el resto calcula un valor incorrecto).
- **Jacobiano del sistema**: cada archivo `.formula` produce además `{filename}__jacobian(all params)`,
  filas = **fórmulas escalares reales que participan en la diferenciación** (los elementos con valor matricial / complejos / no diferenciables se omiten), columnas = parámetros (orden de declaración),
  y una fila que no depende de un parámetro es **0** en esa columna en todo su recorrido. Esta es la forma que necesitan los solucionadores implícitos / las iteraciones de Newton / el ensamblaje FEM.
- **Las constantes no finitas fallan en duro**: emitir `inf` / `NaN` produciría un identificador ilegal,
  así que entra en pánico antes de la generación nombrando el origen (`contains_nan_expr` es la única fuente de verdad).
- **Paréntesis alrededor de potencias**: `(x^2)^3` y `x^(2^3)` reciben paréntesis en ambas superficies —
  `x ^ 2 ^ 3` se reanaliza como `(x^2)^3` (x⁸ → x⁶, es decir, el formateo canónico **cambiaría las matemáticas**).

## Formateo (canónico)

`himari-fmt`: `cargo run -p himari-check --bin himari-fmt -- [--check|--write] <file>`.

- Decls multilínea (un símbolo por línea); una fórmula por línea; **columnas de matriz alineadas** (cada columna = contenido más ancho + ≥1 espacio, la coma como frontera de columna).
- Las líneas de comentario sueltas suben todas a la cabecera del archivo (orden preservado) — los comentarios son para humanos y nunca entran en el IR.
- `--check` = la puerta de deriva (cableada en `just fmt-check` / CI); la salida canónica es idempotente en ida y vuelta.
- **El canónico conserva las comas**: la separación solo por espacios y la resta binaria son indistinguibles al reescribir (`-k` se tragaría como término de resta),
  así que la coma es la frontera inequívoca; escribir a mano sin comas está bien, el formateo las repone.

## Carriles (decisión del usuario del 2026-11-23)

| carril | uso | adecuado para |
| --- | --- | --- |
| **macro en línea** | `formula_matlab!{ decl { a : l [m]; b : l [m]; } a / b }` | fórmulas simples |
| **macro de cadena** | `formula_latex!{ r"decl { … } \frac{a}{b}" }` | fórmulas simples (superficie LaTeX) |
| **macro de archivo externo** | `formula_file!("formulas/bv.formula", matlab)` | fórmulas complejas (el dialecto debe darse explícitamente) |
| **build.rs** | véase `packages/check/fixtures/three/` | código generado versionado + puerta de deriva (el carril formal) |

El `decl` dentro de una macro solo aporta información de símbolos para la comprobación de dimensión; **no crea variables locales de Rust**: `a/b` en el
punto de llamada debe ser ya `let a = …; let b = …;` por parte del llamador. Para un ejemplo compilable completo véase
`inline_macro_example()` en `examples/common/src/main.rs`.

> ⚠️ Limitación conocida: una proc-macro que lee archivos **no es rastreada por cargo** (`tracked_path` sigue en nightly),
> así que tras editar un `.formula` se necesita un `touch` para disparar la recompilación; para producción use el carril build.rs (`rerun-if-changed`).
> ⚠️ El **carril de constructor** para matrices de patrón/transpuestas solo existe para archivos / build.rs (necesita un nombre de archivo y un generador);
> las macros en línea informan un error explícito con una indicación cuando se encuentran con ellos, nunca se degradan en silencio.

## Lenguajes de superficie (tres, un solo IR)

| | artículo | LaTeX | estilo MATLAB |
| --- | --- | --- | --- |
| letras griegas | `α·x` | `\alpha \cdot x` | `alpha*x` |
| elipsis | `⋯ …` | `\cdots \vdots \ddots` | `...` |
| matriz | `[a, ⋯, b; ⋮ ⋱ ⋮; c, ⋯, d]` | `\begin{bmatrix}…\end{bmatrix}` | `[a, ..., b; ...]` |
| traspuesta | `A'`, `A′`, `A^T` | `A^T` | igual que a la izquierda |
| decls de dimensión | `decl { v : velocity [m/s]; }` | igual que a la izquierda | igual que a la izquierda |

Reglas (el rechazo es una característica):

- Todo símbolo debe declararse primero con su dimensión; no declarado = error de compilación, sin inferencia a partir de glifos.
- La notación ambigua o puramente tipográfica como `\sin^2 x`, `\left(\right)`, `$…$`, `x .* y` se rechaza de plano.
- La multiplicación implícita solo se cumple en las superficies LaTeX/artículo (el sistema de una sola letra es inequívoco por construcción); la superficie MATLAB exige un `*` explícito.
- Las matrices de patrón solo soportan tres formas demostrablemente correctas: `identity(n)`, `diag(d, n)`,
  `tridiag(a, b, c, n)`; cualquier cosa no reconocida = error + la lista de esas tres.
- Las fuentes pueden escribirse a mano (marcas combinantes, etc.), pero **el IR debe estar normalizado** — los caracteres combinantes nunca entran en el IR.

## Álgebra vectorial / Horner / no_std (2026-11-23)

**`dot(a,b)` / `norm(v)`**: se expanden solo sobre **literales concretos** (forma conocida en tiempo de compilación);
la dimensión de `dot` es el producto de ambos lados, `norm` tiene la misma dimensión que sus elementos. `dot([x,y],[x,y])` (x=3,y=4) → 25,
`norm` → 5. Las formas que no coinciden se rechazan de plano.

**Conversión automática de Horner**: un **polinomio explícito** escrito como `c₀ + c₁x + c₂x² + …` en la fuente se convierte antes de la emisión en
`(…((cₙx + cₙ₋₁)x + …) + c₀)` — la cuenta de multiplicaciones baja de `2n-1` a `n`, y las potencias grandes como `x^n` desaparecen
(numéricamente más estable). `a*x^3 + b*x` → `(a * x * x + b) * x`.
**Las formas compactas no se expanden**: `(u-1)^3` se queda tal como se escribió (expandirla solo la hace más difícil de leer); las derivadas también se toman sobre
el **original anterior a Horner** (diferenciar la forma de Horner produce una cadena de productos que se cancelan).

**Carril `no_std`**: `himari_buildhelper::generate_dir_libm` produce código que usa solo `core` + `libm`
(trascendentes → `libm::exp` etc., potencias enteras → `libm::pow`; `f64::powi` es cosa de std).
Eso se refiere a que el **código generado** es usable en `no_std` — y desde el 2026-11-23 la pila de compilación también lo es:
`himari`, `himari-latex` y `himari-matlab` son `no_std + alloc` detrás de una feature `std` por defecto (`std` está activa a menos que la desactive).
El cargador, el buildhelper, las macros y las crates de comprobación (E/S de archivos, host de proc-macro, herramientas) siguen en `std` y activan la feature explícitamente.
El código generado conserva **su propio carril libm**, contra el que un consumidor bare-metal realmente enlaza.
Puerta: una crate fixture `#![no_std]` (`[lib] test = false`, porque el arnés de pruebas necesita std) que a la vez compila
código generado **y** ejercita la cadena de analizadores (`himari_matlab::parse` → `dims_of_set` → `shapes_of_set` → `dim_check_runtime` → `format_set`)
sin std. **Que `cargo build` pase = el código generado y el front-end no buscan std en secreto.**
Objetivos bare-metal verificados: `riscv32imc-unknown-none-elf` (solo core — sin atómicos disponibles), `thumbv7em-none-eabi`, y
`x86_64-unknown-none` (cadena de analizadores). Este carril no emite jacobiano (necesita `alloc`) ni variante f32
(los trascendentes f32 son la familia `*f` de libm, y nos negamos a enviar medio carril); para parciales, use las funciones `__d_*` por fórmula
(escalar puro, sin asignación).

## Bibliotecas de fórmulas entre archivos: `use "..."` (2026-11-23)

```text
use "lib/atmosphere.formula"      % file header, may span several lines
decl { ... }
v_terminal = sqrt(2 * m * g / (rho * Cd * A))
```

- **El análisis solo recopila, la E/S pertenece al cargador**: el front-end sigue siendo una API puramente de cadenas (`FormulaSet.uses`),
  y `himari-loader` se encarga de leer archivos, la inclusión recursiva, la detección de ciclos y la resolución de conflictos.
- Semántica de fusión (simple y predecible): los decls/elementos referenciados se **colocan primero** → la federación funciona como siempre (los elementos posteriores pueden referenciar a los anteriores);
  los decls idénticos duplicados se deduplican, distintas grafías dan error; los nombres de elementos duplicados siempre dan error (**sin «gana el último que escribe»**);
  los ciclos dan error e imprimen la cadena de nombres de archivo alrededor del ciclo.
- **Una biblioteca se «sustituye», no se «llama»**: una vez que `rho` se expande en el llamador, la lista de parámetros contiene `rho_0 / h / H`.
  Esto es deliberado (sin estado oculto, diferenciable estáticamente); el coste es una lista de parámetros más larga.
- Convención de directorios: `generate_dir` escanea solo los `*.formula` de **primer nivel**, así que `formulas/lib/*.formula`
  existe naturalmente solo como biblioteca y nunca se genera dos veces como fórmula de primer nivel.
- **El dialecto viene de la macro, no de una adivinanza** (decisión del usuario del 2026-11-24, «la macro es el dialecto»): la macro que envuelve la fórmula
  decide qué gramática la lee.

  | macro | dialecto |
  | --- | --- |
  | `formula!(…)` | **matex** (superficie canónica; se conserva el nombre de entrada, la semántica queda fijada) |
  | `formula_latex!(…)` | LaTeX |
  | `formula_matlab!(…)` | matex (sinónimo de `formula!`, más autoexplicativo de leer) |
  | `formula_file!("f.formula", latex)` | lo decide el segundo argumento **obligatorio** |

  `formula_file!` no hace opcional el argumento de dialecto: el contenido del archivo no se puede inferir del nombre de la macro, y hacer que la macro adivine
  solo escondería la ambigüedad en otro sitio.

  **Por qué esto no es solo una preferencia de estilo** — elimina un valor incorrecto silencioso real. Cuando `si` está declarado, `si * x`:
  matex lo lee como `si·x`, LaTeX lo lee como `s·i·x`. Medido en el mismo archivo con las mismas entradas (s=3, i=5, x=2, si=100):

  ```text
  old AUTO (try matex first, use if it succeeds)  →  200    without reporting any error
  explicit LaTeX                                  →  30
  ```

  Los dos difieren en casi un factor de siete. AUTO no tenía forma de saber qué quería decir el autor — lo que «adivinaba» era simplemente qué analizador
  tenía éxito primero. Por eso la enumeración `Dialect` del carril de macros **no** tiene variante `Auto`: el sistema de tipos garantiza que la adivinanza no puede volver.

## Inicio rápido: `examples/reentry` (un proyecto mínimo ejecutable)

```text
examples/reentry/
  formulas/entry.formula       % terminal velocity / dynamic pressure / drag (aerospace side)
  formulas/structures.formula  % static displacement = solve(k·x − F, x)
  build.rs                     % three lines: generate_dir("formulas", "src/generated.rs")
  src/main.rs                  % prints the table + physical invariant self-checks
  src/generated.rs             % the checked-in generated code (reviewable, covered by the drift gate)
```

`cargo run -p himari-example-reentry` imprime una tabla velocidad→presión dinámica→resistencia y verifica:

- **Invariante físico**: `drag(v_terminal) == m·g` (diferencia relativa 1.6e-16 — esto no es un ajuste, es la definición)
- **Derivada analítica frente a cociente de diferencias**: `∂drag/∂v` coincide con una diferencia relativa de 1.2e-8 entre los dos caminos
- **Resolución simbólica**: `x_static` viene de `solve(k·x − F, x)`, y `∂x/∂F = 1/k` se cumple automáticamente
- **La variante f32** y el **jacobiano del sistema de una fórmula incluida desde una biblioteca** están ambos en el mismo archivo generado

Un consumidor ahora solo necesita `himari-buildhelper::generate_dir(dir, out)` — todo el carril build.rs
(análisis / federación / dimensiones / formas / derivadas / f32 / entradas por lotes / jacobiano / LaTeX fuente) se reduce a una sola función,
así que cada proyecto ya no copia 300 líneas.

## Ejemplos comunes: `examples/common` (generación real + referencias numéricas independientes)

```bash
cargo run -p himari-example-common
cargo test -p himari-example-common
```

| archivo de fórmula | ejemplo | entrada generada y verificación |
| --- | --- | --- |
| `matrices.formula` | rotación, traspuesta, `AᵀA`, inversa 2×2, `A·A⁻¹`, matriz por columna | `rotation_orthogonality` / `N` / `stiffness_inverse` / `nodal_forces`; celda a celda contra el cálculo a mano |
| `springs.formula` | sistema de equilibrio de resortes de tres nodos `solve([…],[u₁,u₂,u₃])` | `displacements` contra una eliminación gaussiana con pivoteo parcial en Rust separada |
| `runtime_matrix.formula` | `A`/`b` en tiempo de ejecución: `solve(A,b)`, `A*x`, `inv(A)`, indexación basada en 1 | `runtime_displacements` / `runtime_traction` / `runtime_inverse` / `runtime_element`; cada llamada falible (`Result`), pivoteo demostrado en `[[1e-17,1],[1,1]]` |
| `least_squares.formula` | ecuaciones normales de un ajuste lineal | `line_fit` contra la forma cerrada directa de mínimos cuadrados, dando `y=1.04+1.99x` |
| `roots.formula` | la cuadrática y la raíz pequeña para `b²≫4ac` | `quadratic_roots` comprueba `1,2` y `10⁻⁸,10⁸` (protección contra la cancelación); devuelve `Result`, y un discriminante negativo es `NumericError::NoRealRoots` |
| `circuit.formula` | `I=V/(R+j·2πfL)` | `current` / `current_magnitude` comprueban `|I|=0.995037 A`; `2π` se escribe explícitamente |
| `dynamic_matrix.formula` | stencil tridiagonal dinámico `n×n` | `diffusion_stencil(k,n)` celda a celda contra una referencia construida con un bucle |
| macro en línea (`src/main.rs`) | `formula_matlab!` con `a/b`, y `solve(a, b)` sobre formas en tiempo de ejecución | variables locales de Rust pasadas de verdad; `m/m=1` comprobado en tiempo de compilación; la resolución en tiempo de ejecución comprobada numéricamente |

Estos ejemplos prueban que las **formas actualmente soportadas** van de verdad del `.formula` a Rust ejecutable; no presentan
`solve` como un solucionador numérico general, y los ejemplos estáticos no requieren parámetros matriciales en tiempo de ejecución.
El `src/generated.rs` generado está versionado y protegido por la puerta de deriva.

## Carril de normas: `himari-standards` (2026-11-23)

`packages/standards` convierte un puñado de **normas de ingeniería publicadas** en calculadoras tipadas, una relación a nivel de cláusula por módulo, y cada una
lleva su propio `PROVENANCE` (id de la norma, edición, la cláusula de la que viene la fórmula, y qué *no* se modela). Nada aquí cita
la prosa de una norma, y nada reclama conformidad con la norma — reformula la fórmula como una expresión simple. La conversión de unidades está
deliberadamente ausente: los llamadores pasan unidades base SI.

| módulo | norma | edición | calculado |
| --- | --- | --- | --- |
| `iso_6506_1_2014` | ISO 6506-1 | 2014 | dureza Brinell `HBW` (más la presión media de huella en Pa) |
| `iso_6892_1_2019` | ISO 6892-1 | 2019 | resistencia a la tracción `Rm`, alargamiento porcentual `A`, módulo `E` |
| `iso_2533_1975` | ISO 2533 | 1975 | troposfera `T`, `p`, `rho` y la conversión geopotencial (0 … 11 000 m, y se niega por encima en lugar de extrapolar) |
| `gb_t_231_1_2018` | GB/T 231.1 | 2018 | dureza Brinell `HBW`; edición vigente, sustituye a GB/T 231.1-2009, modifica y adopta ISO 6506-1:2014 |
| `gb_t_228_1_2021` | GB/T 228.1 | 2021 | resistencia a la tracción `Rm`, alargamiento `A`, módulo `E`; edición vigente (en vigor desde el 2022-07-01) |
| `gb_t_8170_2008` | GB/T 8170 | 2008 | redondeo decimal, **redondeo a par**, decidido sobre cadenas decimales — `f64` nunca participa en una decisión de redondeo |

Los dos módulos GB de ensayos mecánicos reutilizan la aritmética ISO (la misma evaluación probada) pero conservan su **propio** id, edición y procedencia, así que una
respuesta nunca puede etiquetarse con la norma equivocada. Para `GB/T 228.1-2021`, el grado de adopción de cualquier documento ISO **no** se verificó
de forma independiente, así que el módulo no reclama explícitamente ninguna equivalencia — solo se implementan las tres definiciones. Las ediciones se comprobaron contra la
Plataforma Pública de Servicio de Información sobre Normas Nacionales, y cada módulo registra esa comprobación en sus notas de procedencia.

La CLI `himari-standard` evalúa una solicitud e imprime un documento de respuesta de forma estable (JSON), leyendo la solicitud como JSON o TOML —
desde `--input <path>` o stdin, con `--format json|toml` para forzar el analizador y `--list` para los ids y ediciones exactos:

```bash
himari-standard --input packages/standards/examples/brinell.json
himari-standard --input packages/standards/examples/tensile.toml
himari-standard --list
```

Esas tres solicitudes de ejemplo bajo `packages/standards/examples/` son ejercitadas por la suite de pruebas,
así que las invocaciones documentadas no pueden pudrirse.

Las claves desconocidas se rechazan en lugar de ignorarse; cada fallo devuelve el mismo documento con `values` en `null` y la causa en `errors`
(código de salida 1, con un diagnóstico de una línea en stderr); el código de salida 2 está reservado para un mal uso de la línea de comandos. Las solicitudes cuyos decimales participan en una **decisión
de redondeo** deben llevar esos números como cadenas decimales, porque un flotante binario ya ha perdido el último dígito cuando la crate lo ve.

## Carril de demostración: `himari-lean` (2026-11-23)

`packages/lean` es un **co-proceso** de Lean 4: genera una obligación de demostración autocontenida, se la entrega a un ejecutable `lean` real por stdin
(`lean --stdin --json -T <heartbeats> -M <MB>`), e informa lo que pasó en un documento estructurado. Sin proyecto Lake, sin índice de paquetes,
sin red; el archivo generado no tiene imports.

- **Los datos dinámicos solo se convierten en enteros literales** de un *enunciado de teorema de ejemplo con nombre* — nunca en una definición de datos, un axioma o un identificador.
- **Comprobado por el núcleo, con `decide`**: la obligación se cierra sobre `Int` y se descarga con `by decide`. No se emiten `native_decide`, `#eval`,
  `implemented_by`, `extern`, `partial`, `sorry` ni axiomas de usuario, y una prueba afirma el texto generado contra esa lista.
- **Puerta de axiomas**: la salida de `#print axioms` se analiza y solo se toleran `propext`, `Classical.choice` y `Quot.sound`; un informe ilegible es una
  violación, nunca un aprobado. La ausencia de un informe no es evidencia de uno limpio.
- **Un oráculo independiente**: el mismo determinante se calcula en Rust con una expansión *distinta*, así que Lean nunca es el único testigo.
- **`lean_missing` es un no-éxito de primera clase**: significa que la cadena de herramientas no estaba y **no se comprobó nada**. Nunca se muestra como
  «verificado» y nunca sale con 0. Estados y códigos de salida: `success` 0, `lean_missing` 3, `invalid_request` 4, `proof_failed` 5,
  `resource_exhausted` 6, `axiom_violation` 7, `oracle_mismatch` 8.

**Verificado contra una cadena de herramientas real (Lean 4.36.0, commit `611c117b`).** La obligación del hito es aceptada por el núcleo e informa de
ningún axioma en absoluto:

```text
status = success            verified = true       exit = 0
lean_message: 'det3_example' does not depend on any axioms
oracle:       rust-i128-permutation det3([1,2,3,4,5,6,7,8,10]) = -3
```

Los controles negativos se comportan como se diseñó con el mismo binario: una solicitud que afirma `expected = -306` devuelve `oracle_mismatch` (código de salida 8) y
nunca se informa como verificada, y la suite de procesos simulados cubre los caminos `proof_failed` / `axiom_violation` / `resource_exhausted`.

El aprovisionamiento es externo a este repositorio y sigue siendo puntual: instale `elan`, fije `leanprover/lean4:v4.36.0`, y luego ponga `lean` en el `PATH`
o apunte `HIMARI_LEAN_BIN` a él. `just lean-verify` emite la obligación y ejecuta la prueba sobre Lean real. **Sin cadena de herramientas el carril se degrada a
`lean_missing` (un no-éxito) y `--emit` aún escribe un archivo `.lean` revisable** — la ausencia de Lean nunca se informa como verificación.

## Robustez y entrada adversaria (2026-11-23)

- **Límite de profundidad 2048**: cada salida del análisis comprueba una vez la profundidad del IR (`check_depth`), y todo lo que supera el límite informa
  «fórmula anidada demasiado profunda (N levels > limit 2048)» en lugar de dejar que alguna recursión posterior desborde la pila (SIGSEGV).
  El propio límite de pila de pest también salta y su mensaje se traduce también a palabras llanas. **Frontera conocida**: una cadena larga `a + b + c + …` está
  anidada a la izquierda en el IR, así que la profundidad es igual al número de términos; el número de términos de una fórmula queda por tanto limitado a unos 2048.
  Una guarda determinista separada contra el anidamiento de paréntesis (256 niveles, `MAX_PARSE_NESTING`) se ejecuta antes del análisis, porque también debe cumplirse donde la
  feature opcional `stack-guard` de pest no está disponible (`no_std`).
- **Los caracteres invisibles siempre se rechazan**: espacio de ancho cero / override bidireccional / joiner de ancho cero / BOM / NUL son todos
  nombrados y rechazados antes del análisis (`scan_source`) — pueden hacer que una fórmula «parezca distinta al revisarla de lo que es».
- **Límite de pasos de suma/integral 10^7**: `Σ(i,1,1e9)` pasa de «se ejecuta toda la noche» a una línea de error.
- **Una llamada de función con argumentos faltantes** ya no entra en pánico por la indexación `args[0]`: los tres carriles (evaluación/emisión/dimensión) la rechazan explícitamente.
- **Pruebas de propiedades estructuradas** (no sopa de caracteres aleatoria): 600 expresiones aleatorias pasan cada una tres puertas — la reescritura canónica se reanaliza al
  **mismo árbol**, la reescritura LaTeX se reanaliza al **mismo árbol**, y `simplify` **preserva el valor**.
  Las tres muestran actualmente 0 no-equivalencias (`prop-roundtrip`); una entrada grande que suma 1500 términos termina en menos de 100 ms.

## Carril de precisión simple (f32) (2026-11-23)

Cada fórmula **escalar real** produce además `{name}__f32(p: f32) -> f32` — la misma fórmula,
las mismas reglas de emisión, con solo los literales pasados al sufijo `f32` (`Fx::F32`, el único interruptor enhebrado por el emisor).
f32/f64 medido = **1,44×** (`SIMD` 8 carriles frente a 4 carriles + la mitad del ancho de banda),
de lo que los motores de juego / el DSP de audio / la SDR en tiempo real se benefician directamente.

Las fórmulas complejas y con valor matricial **no** producen variante f32: la tupla `(f64, f64)` del carril complejo y el
`Vec<Vec<f64>>` de la matriz son un contrato de tipo distinto en precisión simple; eso puede venir después (nos negamos a enviar medio carril).

## Herramientas para desarrolladores (2026-11-23)

Se envían tres puntos de entrada juntos (todos en la crate `himari-check`, compartiendo un solo analizador):

| punto de entrada | propósito |
| --- | --- |
| `himari-fmt --diff` | mostrar solo la diferencia entre la forma canónica y la actual (sin reescribir) |
| `himari-fmt --diagnose` | solo diagnósticos de análisis + dimensión + forma (silencioso con código de salida 0 cuando está limpio) |
| `himari-fmt --symbols` | listar los nombres `decl` y los nombres de fórmula (la fuente de datos para la completación del editor) |
| `himari-eval <file> --set k=v …` | **evaluar**: `Re = 200000 [1]`; `--groups` también imprime los grupos Π de Buckingham; `--json` para máquinas |
| `python3 scripts/fmt_check.py --fix` | un solo comando para rehacer la forma canónica + regenerar los archivos auxiliares |

**Los errores llevan caret**: `FrontError::render_with_source` imprime la línea de fuente y dibuja un `^` bajo la columna culpable
(los tres carriles — build.rs / fmt / eval — pasan por él) — ya no hay que contar hasta la columna 17 uno mismo.

**La extensión de VS Code** (`packages/vscode`) tiene dos canales de comunicación:

| canal | disparador | fuente de datos |
| --- | --- | --- |
| CLI (por defecto, instalación cero) | abrir / guardar | `himari-fmt --diagnose` + `--symbols` |
| **LSP** (opcional) | **errores mientras escribe** | `himari-lsp` (stdio JSON-RPC) |

`himari-lsp` es un servidor mínimo escrito a mano (sin dependencia de un framework LSP): `initialize` /
`didOpen` / `didChange` / `completion` / `codeAction` / `shutdown`, con **sincronización incremental de texto**.
Sus diagnósticos portan el mismo `himari::Error` que la biblioteca, representados en el idioma que elija `HIMARI_LANG` (con reserva en `LC_ALL`, `LANG` y luego inglés); los nueve idiomas del proyecto están disponibles.
La capa de protocolo trata el estado y los parámetros JSON-RPC como funciones puras, y hay una prueba de extremo a extremo que realmente arranca un proceso y realmente envía
tramas `Content-Length`. Configure `himari.lspPath` en los ajustes para cambiar (déjelo vacío = canal de comunicación CLI).

Candidatos de completación = los nombres `decl` del archivo + los nombres de fórmula + la tabla de funciones unarias (incluyendo `det/inv/solve/dot/norm` etc.).

## Superficie LaTeX: los nombres de varias letras necesitan `\mathrm` (2026-11-23)

En modo matemático LaTeX un `sigma` desnudo son **cinco variables multiplicadas s·i·g·m·a** (multiplicación implícita),
así que los nombres de varias letras en este dialecto deben escribirse `\mathrm{sigma_0}` — y la reescritura los emite de la misma manera.
Los nombres de letras griegas (`sigma`, `omega`…) se reescriben como los **glifos reales** `\sigma`, conservando el subíndice
(`\sigma_0`). La implementación solía ser sustitución de subcadenas, así que `alphabet` se convertía en `\alphabet` y
`betamax` en `b\etamax`, mientras que `sigma` se renderizaba como cinco letras en cursiva — mal en tres sitios a la vez (hover / archivo auxiliar / fuente de verdad de la documentación). Corregido (`latex_var`).

## Errores: una sola enumeración, nueve idiomas

Cada fallo del crate es una única enumeración, `himari::Error`. No es un saco de cadenas: cada fallo
es una variante que **lleva sus datos**, lo que da tres cosas a la vez.

- **Un código estable** — `err.code()` devuelve un identificador en kebab-case (`dimension-mismatch`,
  `undeclared-symbol`, `no-real-roots`, ...). Las pruebas y los llamadores verifican el código, así que
  un cambio de redacción o una traducción nunca pueden romperlos.
- **Una representación por idioma** — `err.render(locale)` y `Display` (inglés) producen la frase a
  partir del catálogo de [i18n.rs](packages/himari/src/i18n.rs). `HIMARI_LANG` / `LC_ALL` / `LANG`
  seleccionan el idioma en los bordes CLI y LSP; las nueve locales son las de la organización
  (`en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `fr`, `es`, `ru`, `ar`).
- **Compatibilidad con `anyhow`** — `Error` implementa `core::error::Error` (por tanto
  `std::error::Error` bajo `std`) y es `Send + Sync + 'static`, sin ningún envoltorio:

```rust
  use anyhow::Context as _;

  fn run() -> anyhow::Result<f64> {
      let dim = himari::parse_unit("N/m")?; // `?` converts automatically
      let e = himari::Error::NoRealRoots { discriminant: -4.0 };
      let err = anyhow::Error::from(e).context("solving the loop equation");
      let root = err.downcast_ref::<himari::Error>().expect("root cause kept");
      assert_eq!(root.code(), "no-real-roots");
      assert_eq!(dim, himari::parse_unit("kg/s^2")?);
      Ok(1.0)
  }

  fn main() {
      assert_eq!(run().unwrap(), 1.0);
  }
```

`tests/anyhow_compat.rs` fija todo esto: la propagación con `?`, la cota `Send + Sync + 'static`,
que `downcast_ref::<himari::Error>()` recupere el código tras `anyhow::Context`, y la regla de que
el mismo error se representa distinto en los nueve idiomas mientras el código no cambia.

`Error::Message(String)` sigue existiendo para los mensajes que nunca entran al catálogo; se muestra tal cual en todos los idiomas. La deuda de puesta en espera que representa se **mide con una puerta**, no con prosa — y esa deuda ahora mismo es cero:

```text
python3 scripts/staging_budget.py                # counts + ceiling
python3 scripts/staging_budget.py --against-git  # also: the ceiling may only go down vs the merge base
```

Es un recorrido del AST (`tools/staging-budget`, sobre `syn`): el recuento no depende del diseño de líneas, espacios ni comentarios, y un *patrón* como `matches!(e, Error::Message(m) if …)` nunca se confunde con una construcción. Se cuentan doce cubos en tres familias en cada árbol de producción del espacio de trabajo (`packages/`, `examples/`, `tools/`): las seis formas de construcción literales (`Error::from(`, `Error::Message(`, y en los front-ends `FrontError::plain("…")` / `plain(format!(…))` / `FrontError::new(…, "…")`), los embudos de lavado de valores (`plain` / `new` con una carga útil de valor en tiempo de ejecución, y `FrontError::plain` pasado como referencia de función, ambos en todo el repositorio), y las formas de puesta en espera de cadenas (`ok_or`/`ok_or_else`/`map_err` que producen cadenas en espera, y `Err(format!(…))`) dentro del propio crate del catálogo. Los ítems compilados solo bajo pruebas y el catálogo se omiten; una variante catalogada construida con una carga útil de cadena (p. ej. `Error::SyntaxError(text)`) no es puesta en espera y no se cuenta. La puerta falla si el total supera el techo registrado (el techo vive en `tools/staging-budget/src/ceiling.txt` y solo puede bajar; ahora está en `0`). El techo solo puede bajar, y eso se aplica: CI (y `just verify`) ejecutan la puerta con `--against-git`, que lee `tools/staging-budget/src/ceiling.txt` en la **base de fusión con el tronco remoto** y falla si el techo del árbol de trabajo es más alto — así que no se pueden colar sitios nuevos junto con un techo elevado. El comando pelado de arriba solo comprueba el recuento. Las 51 variantes del catálogo quedan fijadas por `catalogue_completeness_tests`, que construye las 50 variantes del catálogo más la `Message` de puesta en espera y las representa en los nueve idiomas (no vacías, sin `{n}` que se filtre). Los *títulos* de corrección rápida del LSP siguen en chino; sus diagnósticos están localizados.

### Integración con el editor (LSP)

`himari-lsp` es un servicio LSP escrito a mano (la capa de protocolo son funciones puras, con pruebas unitarias + una prueba stdio de extremo a extremo):

| capacidad | descripción |
| --- | --- |
| diagnósticos | empuja `publishDiagnostics` tras `didOpen`/`didChange` (símbolos no declarados, discrepancias de dimensión, errores de sintaxis) |
| completación | los nombres `decl` del archivo + los nombres de fórmula + la tabla de funciones unarias (`det/inv/solve/roots/...`) |
| **arreglo rápido** | `textDocument/codeAction`: símbolo no declarado → ① reescribir al nombre declarado más cercano ② añadir una línea al bloque decl; nombre de función mal escrito → el nombre del registro |
| sincronización | **sincronización incremental** (`textDocumentSync: 2`, cortada por rango); también se aceptan cambios completos, así que los clientes antiguos no se rompen |

Un arreglo rápido solo ofrece un reemplazo cuando el nombre aparece **exactamente una vez** (si aparece varias veces solo ofrece «añadir la declaración», nunca
adivina cuál ocurrencia); los valores fuera de rango siempre se **acotan** en lugar de descartar el cambio (descartar uno
desincronizaría permanentemente el búfer del editor).

### Bloqueos de contenido de bibliotecas de fórmulas: cuando una biblioteca cambia, debe decirlo

`use "lib/…"` **incluye** el archivo de biblioteca, así que cuando la biblioteca cambia, el código generado aguas abajo cambia con ella — exactamente el tipo de
regresión más fácil de pasar por alto. `--lock` escribe una huella de cada archivo referenciado en `<entry>.lock`, y a partir de ahí **el bloqueo se verifica en cada carga**:

```text
$ himari-fmt --lock formulas/entry.formula
已写入 formulas/entry.formula.lock（4 行）

$ himari-eval formulas/entry.formula         # someone quietly changed the library
himari-eval: 公式库与锁文件不一致（formulas/entry.formula.lock）：
  内容变了（声明版本 1.4.2 → 2.0.0）：lib/atmosphere.formula
              复核改动后运行 `himari-fmt --lock <入口>` 重新生成锁。
```

(La salida de la CLI se cita literalmente: la herramienta aún imprime en chino. En inglés: la
biblioteca y su archivo de bloqueo no coinciden — `lib/atmosphere.formula` cambió su versión declarada 1.4.2 → 2.0.0 — vuelva a ejecutar
`himari-fmt --lock <entry>` tras revisar el cambio.)

- **Hash de contenido sha256** (no un rango semver): para una forma de dependencia como «fuente incluida», un hash de contenido es la
  garantía más fuerte — cualquier cambio se atrapa, no se necesita política de resolución de versiones, ni reglas de pre-lanzamiento.
- **Texto plano, determinista, rutas relativas al directorio de entrada**: puede ir a git, el diff es legible, y es comparable entre máquinas.
- **Puerta cerrada**: `scripts/fmt_check.py` ejecuta `--lock-check` sobre archivos que tienen un `.lock`
  (y vuelve a bloquear automáticamente bajo `--fix`) — un bloqueo obsoleto se pone en rojo en la puerta, sin esperar a que alguien cargue el archivo.
- **opt-in**: sin `.lock` no se verifica nada (cero impacto en repositorios existentes). Una vez que existe un bloqueo, **una nueva referencia también falla**
  (semántica de lista blanca); vuelva a bloquear para aceptarla.
- **Un bloqueo ilegible también es un fallo**: no UTF-8 (guardado como UTF-16 por un editor), permisos insuficientes, o una ruta de directorio
  son todos errores duros — nunca pueden tratarse como «sin bloqueo», lo que apagaría la protección en silencio.
- **`--fix` tiene tres etapas** (formatear todo primero → luego volver a bloquear todo → finalmente regenerar cada archivo auxiliar):
  los archivos auxiliares verifican el bloqueo al cargarse, así que el bloqueo debe ser final primero; ambos también dependen del contenido final de las bibliotecas referenciadas,
  y mezclarlos en un solo bucle deja artefactos obsoletos.
- `%! version: 1.2.0` es solo la columna legible por humanos (está bloqueada y se informa un cambio de versión); la garantía semántica descansa en el hash.
- Por qué no rangos semver: eso necesitaría toda una política de resolución/fijado/pre-lanzamiento, mientras que la forma de dependencia de este repositorio es
  «fuente incluida». Para expresar «permitir una deriva menor», volver a bloquear es la afirmación más directa.

### Directivas de dialecto: la contrapartida del carril de archivos a «la macro es el dialecto»

Un archivo no tiene macro que lo envuelva, así que la regla correspondiente es: **matex por defecto; declarar LaTeX explícitamente.** Ambos carriles comparten un principio —
sin adivinar, el autor (no el analizador) decide la semántica.

```text
%! dialect: latex
decl {
    s : d;
    i : d;
    si : d;
    x : d;
}
q = si * x        % LaTeX semantics: s·i·x (not the declared si·x)
```

- Soporta `%! dialect: latex` (o `tex`) y `%! dialect: matex` (o `matlab`).
- **Sin directiva = matex**. Aquí había antes un respaldo AUTO (si el análisis matex fallaba y la fuente contenía `\`, probar LaTeX),
  y producía exactamente el valor incorrecto silencioso de 200 frente a 30 de arriba — se ha eliminado.
- Notación solo LaTeX sin directiva de dialecto → analizada como el matex por defecto → **error** (nunca un valor incorrecto silencioso);
  el error ofrece una salida (escribir `%! dialect: latex`) pero **no adivina en su nombre**.
- Solo surte efecto **antes de la primera línea no comentario** (un `%!` dentro de comentarios del cuerpo no cambia el dialecto).
- Un valor no reconocido es un error.
- Con una directiva, los errores se informan en ese dialecto: un error de sintaxis bajo una directiva LaTeX es un error LaTeX.
- Está implementado en exactamente un lugar (`himari_loader::parse_source`), así que el carril build.rs, `himari-fmt`,
  `himari-eval` y los diagnósticos LSP coinciden todos. (los diagnósticos del LSP se representan en el idioma que elija `HIMARI_LANG`; véase la sección «Errores».)

### División en la superficie LaTeX (en su día ausente)

`a / b` y `a \div b` son ahora división real, sinónimos de `\frac{a}{b}`, con la misma precedencia que la multiplicación y asociativos por la izquierda.
Antes la superficie LaTeX **no tenía ningún operador de división** — solo se aceptaba `\frac{a}{b}`, y `a / b` era un error de sintaxis.
Era tirar al niño junto con el agua de la bañera: `/` es la forma más corriente de escribir una división y no había razón para rechazarla.
(El respaldo AUTO de arriba de hecho había estado tapando este defecto: hacía que `a / b` «funcionara por casualidad» trayendo consigo el valor incorrecto silencioso.
Arregle el defecto y el respaldo no tiene razón de existir.)

## Resolución lineal simbólica `solve` (2026-11-23)

La forma de una variable `solve(equation, variable)` resuelve `equation = 0` como `variable = -b/a` (la ecuación debe ser **lineal** en esa variable).

```text
ES_ss = solve(k_on * E * S - (k_off + k_cat) * ES, ES)
      → ES_ss = k_on·E·S/(k_off + k_cat)          % Michaelis–Menten steady state
```

- La solución **sigue siendo IR**: así conserva la comprobación de dimensión, la evaluación, la generación de código, las derivadas analíticas y
  el jacobiano — no se necesita un segundo mecanismo. Las parciales y las filas del jacobiano de `ES_ss` salen automáticamente.
- **La lista de parámetros = las variables que la expresión emitida realmente usa**: el `ES` resuelto no aparece en los parámetros de `ES_ss`
  («pedirlo y aun así tener que pasarlo» está arreglado). El archivo auxiliar / el hover / la completación leen la misma lista de parámetros.
- **La no linealidad se rechaza explícitamente**: `x·x`, `1/x`, `x^2`, `sin(x)` dan error **en la salida del análisis** y nombran el tipo implicado;
  sin aproximación numérica, sin respuesta dudosa. Equivocarse en un estado estacionario químico cuesta bastante más que teclear unos caracteres más.

### La forma de sistema `solve([equations…], [unknowns…])`

Cuando ambos argumentos son listas se entra en el carril de **sistema lineal**, y la solución es un **vector columna** (el carril de valores de expresión matricial,
devolviendo `Vec<Vec<f64>>`):

```text
x = solve([(k1 + k2) * x1 - k2 * x2 - F1,
           -k2 * x1 + (k2 + k3) * x2 - F2], [x1, x2])
  → x = 2×1 column vector, dimension [m]           % two-mass spring statics
```

- **Eliminación simbólica**: la eliminación de Gauss–Jordan se ejecuta sobre el **anillo de expresiones** (los coeficientes son subexpresiones del IR, no números),
  simplificada a forma normal en cada paso. Cada componente resuelta sigue siendo IR y pasa por dimensiones/evaluación/emisión como de costumbre.
  **Pero una solución de sistema tiene valor matricial**: las fórmulas con valor matricial no generan derivadas analíticas (las parciales elemento a elemento no son una matriz; el jacobiano sí),
  y no generan variante f32 — ese es el límite existente del carril con valor matricial, no algo específico de `solve`.
- **Convención de forma**: las listas de ecuaciones e incógnitas se **aplanan** en orden por filas, así que `[e1, e2]` y `[e1; e2]` son equivalentes —
  los usuarios no tienen que recordar una orientación. Las entradas de la lista de incógnitas deben ser nombres de variable desnudos.
- **Las incógnitas necesitan dimensiones declaradas**: lo que eso aporta es **comprobar la dimensión del resultado** (el ejemplo de resortes resuelve `m = N/(N/m)`),
  así como la comprobación de **homogeneidad** de las propias ecuaciones (`x1 − F1`, sumar m a N, se rechaza en el análisis).
  Las incógnitas resueltas no entran en la lista de parámetros (la función generada solo toma parámetros reales).
- **El rechazo es una característica**: subdeterminado (no rango completo), inconsistente (tras la eliminación `0 = nonzero`), no lineal (la incógnita aparece a ambos lados de una
  multiplicación / en un denominador o exponente / dentro de un argumento de función) se **informan todos en la salida del análisis**.
  Los rechazos relacionados con dimensiones ocurren en `dims_of_set` / la etapa de generación (el propio analizador no hace comprobación de dimensiones).
  Las ecuaciones redundantes se eliminan «en el sentido plegable»: una fila redundante con coeficientes simbólicos que no se pliega a cero se informa de forma conservadora
  como inconsistente — mejor rechazar que devolver una solución incorrecta.
- **Pivote no nulo y la frontera numérica**: misma convención que `solve` de una variable, algo que no se puede probar nulo se trata como
  no nulo (confiamos en `a ≠ 0`). La eliminación ocurre en tiempo de compilación en el orden de las expresiones y **no reordena filas por |pivote| en tiempo de ejecución**;
  a escalas extremas la forma cerrada se cancela, un parámetro singular informa una división por cero en la capa de valores y da `inf/NaN` según IEEE en
  el código generado — eso no es una excepción. Los sistemas mal condicionados, o los que necesitan una estrategia de pivoteo, deben ir a una biblioteca numérica.
- **Fronteras (cuatro, todas rechazadas en la salida del análisis o como errores de compilación)**:

  1. Un coeficiente o término constante **con valor matricial** (`solve(M*x - b, x)`, `solve([M*x - b], [x])`) → rechazado explícitamente con la
     indicación de `inv(A) * b` — si no, el carril escalar generaría una división `vec!`, que no compila.
  1. **Las ecuaciones pueden tener dimensiones distintas** (un equilibrio de fuerzas en N más una restricción geométrica en m es un sistema legal), pero cada ecuación debe ser
     homogénea por sí misma; y cada componente de la solución debe compartir **una dimensión** (incógnitas de dimensiones mixtas — resolver a la vez una corriente y una
     resistencia — aún no se soportan; es un límite del carril con valor matricial).
  1. El carril complejo (con `j`) funciona en **la capa de valores** y **la capa de emisión** (completado el 2026-11-23): un vector columna complejo se
     emite como `Vec<Vec<(f64, f64)>>`, siendo los elementos (real, imaginario). El análisis nodal de circuitos AC y los «sistemas de ecuaciones complejas» de estilo EIS
     pueden por tanto generarse directamente a código.
  1. **La prueba de cero sobre filas redundantes es una condición suficiente**: los ceros que la combinación de términos semejantes puede probar (`k−k`, `k·a−a·k`, `x·x²−x³`)
     se reconocen; una identidad compleja que no se puede probar nula se trata como coeficiente no nulo (conservador — puede informar falsamente
     subdeterminado/inconsistente, pero no devolverá una solución incorrecta).

### Raíces de polinomios `roots(poly, x)`

`solve` da una forma cerrada lineal; `roots` da **todas las raíces de un polinomio** (como vector columna):

```text
roots_quad = roots(a * x ^ 2 + b * x + c, x)
% q = −(b + sign(b)·√(b²−4ac))/2; large root = q/a, small root = c/q
% Return order still matches the original formula (−b−√Δ first); it does not depend on sorting the roots by size.
```

- **Solo grado 1–2**: el grado 1 es la misma matemática que `solve`; el grado 2 es la fórmula cerrada de las raíces.
  **El grado 3 y superiores se rechazan explícitamente** con el grado real informado — Cardano/Ferrari están terriblemente mal condicionadas en coma flotante,
  y este repositorio no entrega una aproximación «que parece precisa». Para raíces de grado tres y superiores, calcule primero los coeficientes numéricamente
  y luego use una biblioteca numérica.
- **Las raíces repetidas se repiten según su multiplicidad** (`x² − 2x + 1` da dos 1), coherente con el teorema fundamental del álgebra;
  el respaldo `q=0` para una raíz nula es `-b/(2a)` y no pasa por `0/0`.
- **Evitar la cancelación en la fórmula cuadrática**: usar `(−b±√Δ)/(2a)` directamente pierde la raíz pequeña cuando `b²≫4ac`
  (`x²−10⁸x+1` en su día calculaba `10⁻⁸` como unos `7.45×10⁻⁹`). Ahora usa las estables `q/a` y `c/q`, y la línea de aceptación para el error relativo de la raíz pequeña
  en el código generado es `1e-12`; ambas direcciones de signo y el carril complejo tienen regresiones.
- **Las dimensiones salen automáticamente**: la ecuación debe ser dimensionalmente homogénea, y la homogeneidad implica directamente «la dimensión de una raíz = la
  dimensión de la variable», así que no se necesita derivación extra de dimensión (una prueba de contrato protege este razonamiento).
- **Discriminante negativo en el carril real**: el código generado devuelve un error tipado, **no NaN** — la entrada real cuadrática `roots` ahora devuelve
  `Result<_, himari::runtime::NumericError>` y un discriminante negativo es `NumericError::NoRealRoots { discriminant }`. Ya no hay un contrato de NaN
  silencioso, así que las comprobaciones `is_nan`/`is_finite` ya no son la forma de detectar este caso; un discriminante constante
  demostrablemente negativo en tiempo de compilación sigue rechazándose de plano. `eval_value` es la capa general de valores complejos y **sigue devolviendo raíces complejas**
  para la misma expresión en el carril complejo; un carril complejo necesita un `j` explícito.
- **Los términos de orden superior que se cancelan se eliminan**: `x³ − x³ + x − 1` se trata como grado 1 (no «rechazado porque aparece un x³ en el polinomio»).
- **Los no polinomios se rechazan explícitamente**: `x` dentro de un argumento de función, en un denominador, o a una potencia no entera — cada uno con su razón dada.
- **Los coeficientes con valor matricial** se rechazan (misma convención que `solve`, si no el carril escalar generaría una división `vec!`).

### Vector de coeficientes `poly_coeffs(poly, x)`

El complemento de `roots`: da los **coeficientes** (de grado bajo a alto), devolviendo un vector fila `1×k`.

```text
coeffs = poly_coeffs(2 * s ^ 2 - 3 * s + 1, s)   → [[1, −3, 2]]
```

- **La variable debe ser adimensional**. La dimensión del coeficiente de grado k de un polinomio homogéneo es `dim(poly)/dim(x)^k`, y difieren;
  pero una matriz requiere **elementos de una sola dimensión** — así que tomar coeficientes de una variable con dimensión es algebraicamente imposible en esa forma.
  Esta comprobación lo explica directamente en la etapa de diagnóstico y ofrece una salida (dividir por una magnitud característica, o usar `roots`) en lugar de dejar que
  aguas abajo informe «las dimensiones de los elementos de la matriz difieren».
- Los coeficientes nulos de grado alto se eliminan (`x³ − x³ − 2x + 1` es grado 1); el polinomio nulo se rechaza.
- Los no polinomios (`sin(x)`, `1/x`, potencias no enteras) se rechazan.
- Comparte el mismo extractor `poly_coeffs` que `roots` — una sola definición de la semántica de coeficientes, dos usuarios.

### Raíces por encima del grado tres: `himari-eval --roots`

Rechazar el grado ≥ 3 en el lenguaje es un compromiso forzado, pero la búsqueda **numérica** de raíces funciona para cualquier grado, así que vive en la capa de herramientas:

```text
$ himari-eval cubic.formula --roots
cubic = 1, 2, 3  [1] (degree 3, 3 roots)
cplx  = -1, 0.5-0.8660254037844387j, 0.5+0.8660254037844386j  [1] (degree 3, 3 roots)
```

- **Iteración de Durand–Kerner** (reutilizando la `C` del núcleo para la aritmética compleja, sin biblioteca numérica de terceros): los valores iniciales son potencias de
  `0.4 + 0.9j`, el radio es la **media geométrica de las magnitudes de las raíces** `|c₀/cₙ|^(1/n)` (recurriendo a la
  cota de Cauchy `1+max|cᵢ|` solo en casos degenerados); como máximo 1000 iteraciones; el criterio DK es `|Δz| ≤ 1e-10·radius` (relativo a la magnitud de las raíces,
  no relativo a 1); seguido de **60 pasos de pulido de Newton** (criterio de parada relativo `1e-15·|z|`).
- **La no convergencia es un error**, nunca una aproximación aún no convergida devuelta como resultado; `inf`/`NaN` durante la iteración también es un error inmediato
  (`f64::max` devuelve el otro lado para NaN, lo que en su día hizo que un paso NaN pareciera «convergido» y devolviera 20 NaN).
- **Aceptación por residuo**: tras iterar, el error inverso relativo `|p(z)| / Σ|aₖ|·|z|ᵏ` debe ser lo bastante pequeño (umbral 1e-8),
  si no da error — DK no garantiza la convergencia a una raíz, y el criterio debe ser independiente de la propia iteración.
  El denominador **debe incluir el término `k=n`**: sin él el denominador degenera en la constante `|a_n|` en `|z|` pequeños,
  y `x²−1e-100` pasaría estando equivocado por un factor de 4e25 (una regresión real).
- **Pulido de Newton** (criterio relativo): una raíz simple alcanza la precisión de máquina; **una raíz múltiple converge solo linealmente**, así que `x²−2x+1` y
  similares tienen un error relativo del orden de `1e-9` y no dan error — eso es una propiedad del algoritmo, no un bug; no trate una raíz múltiple como
  una simple.
- **Límite de grado 16** (el límite de expansión de polinomios de `poly_coeffs`; `numeric_roots` en sí soporta hasta 64).
- **Normalización mónica** antes de iterar (dividir por el coeficiente principal) para evitar desbordamiento de magnitud; límite de grado 64; coeficientes `inf`/`NaN` rechazados.
- **La puesta a cero del ruido borra solo la parte imaginaria, y el criterio es relativo**: una raíz real lleva un residuo imaginario de escala `1e-16`, que es un
  artefacto de iteración; el criterio es `|im| ≤ 1e-12·|z|`. **La parte real nunca se toca** (las raíces de `x²−1e-50` son ±1e-25),
  y **un umbral absoluto tampoco sirve** — para una raíz compleja cuya magnitud está por debajo de 1e-12, la parte imaginaria es toda la información de la raíz
  (`x³−1e-60` en su día se difuminó en tres raíces reales sobre esa base, error relativo 0,87, sin error).
- Los coeficientes deben evaluarse a números bajo los parámetros dados por `--set`; cuando se omite la variable se infiere como «el único símbolo declarado en la
  fórmula», y varios candidatos requieren `--var`.
- La salida lleva el grado y el número de raíces; las raíces complejas se escriben `a+bj`.

## Matrices complejas y sistemas de ecuaciones complejas (2026-11-23)

La complejidad no es solo escalar: los elementos de matriz y los vectores de solución también pueden ser complejos, y **la capa de valores y la capa de emisión coinciden**.
El tipo de retorno de la función generada es `Vec<Vec<(f64, f64)>>` (elemento = (real, imaginario)):

```rust
// I = solve([Z*I1 - V, Z*I1 - R*I2], [I1, I2]) with Z = R + jωL
pub fn I(R: f64, w: f64, L: f64, V: f64) -> Vec<Vec<(f64, f64)>> { … }
```

- Los usos típicos son el **análisis nodal de circuitos AC** y el modelado EIS: las incógnitas son corrientes/potenciales complejos.
- La transposición es **transposición simple** (sin conjugación) — la conjugación es el operador separado `conj`; no las confunda.
- `adjoint` es la traspuesta conjugada; solo pueden expandirse los literales cuya forma se conoce en tiempo de compilación.
- En el carril complejo `det/trace/adjoint/inv/solve/roots/dot/norm` **igualmente** se expanden a nivel de IR antes de la emisión (una asimetría histórica:
  el carril real expandía mientras el complejo iba directo a `compile_error!`).
- Aún no soportados: el **carril no_std/libm** para fórmulas complejas (los cierres dependen de std), las **derivadas analíticas** para fórmulas complejas
  (`abs/arg/conj/re/im` no son analíticas complejas, eso necesita cálculo de Wirtinger), y la **variante f32** para fórmulas complejas.

## Carril WGSL (GPU): subconjunto + frontera de validación (2026-11-23)

`himari-fmt --wgsl <file.formula>` emite una fórmula escalar real como kernel f32:

```wgsl
// Generated by himari (f32 scalar lane; really compiled and validated by naga)
fn q(rho: f32, v: f32) -> f32 {
    return 0.5f * rho * (v * v);
}
```

**Subconjunto soportado (deliberadamente pequeño)**: `+ - * /`, `sqrt`, `exp`/`log` (el `ln` de este repositorio) `sin cos tan
sinh cosh tanh abs min max clamp`, más las potencias 2/3, la raíz cuadrada, y la potencia -1.
**Rechazados explícitamente**: complejo (`j`), matrices, `det/inv/solve`, Σ/∫, operadores simbólicos, otras potencias,
y símbolos que colisionan con palabras reservadas de WGSL — cada uno con una razón (no emitimos un kernel que no compile).

**La validación tiene dos capas** (desde el 2026-11-24 el carril WGSL tiene evidencia **a nivel de compilador**):

1. **Puerta de compilación real** (autoritativa): `cargo test -p himari-check --features wgsl-compile`
   entrega cada kernel emitido en las fixtures a **naga** (el compilador de shaders de wgpu, una de las implementaciones autoritativas de la especificación WGSL) para
   análisis real + validación real. naga es una dev-dependency puramente de CPU — **no se necesita GPU**, y nunca entra en el árbol de dependencias en tiempo de ejecución.
1. **Puerta estática** (cribado rápido, `wgsl::validate_wgsl`): equilibrio de paréntesis, una firma `-> f32`, exactamente un
   `return`, literales flotantes que llevan `f`, sin `^`/`**`, sin constantes no finitas. Su valor es «atrapar errores gruesos en entornos sin naga
   + mensajes de error más cercanos a este carril»; **no es una prueba de corrección**.

**Por qué una puerta de compilación real es obligatoria — una lección concreta**: todos los kernels de este carril WGSL eran en su día **inválidos**.
WGSL no tiene funciones con cuerpo de expresión; el cuerpo debe ser `return expr;`, mientras que el emisor siempre emitía una expresión desnuda;
peor aún, la puerta estática **prohibía `return`** — porque la regla se escribió sin conocer esa parte de la especificación, convirtiendo el
malentendido en «la regla». El punto ciego estructural de una puerta estática no es «se le escaparán cosas» sino «**no sabe qué se le ha escapado,
y puede incluso juzgar código correcto como incorrecto**». La primera ejecución de naga expuso toda la clase.
Otra creencia corregida: `return 2.5;` es **legal** en naga (el flotante abstracto aterriza como f32 por contexto) — así que «los flotantes deben llevar `f`» es una convención de estilo
de este carril, no de la especificación WGSL.

**Frontera de validación (declarada honestamente)**: naga valida **solo sintaxis y tipos**. «¿Calcula el kernel los números correctos en una GPU real» sigue
sin verificar — eso necesita un entorno de ejecución wgpu + una GPU (node-3 tiene una RTX 5880, conexión pendiente).
Además, tint (la implementación de Chrome/Dawn) y naga difieren en algunas reglas de borde; este repositorio solo responde ante naga.

## Elegir un carril por dominio (una tabla)

| escenario | carril sugerido | notas |
| --- | --- | --- |
| HPC científico a gran escala / bibliotecas numéricas | `.formula` + build.rs, escalares reales | a la par del escrito a mano (1,0×); la variante por lotes de longitud fija elimina las comprobaciones de límites |
| motores de juego / DSP de audio | igual + **`{name}__f32`** | ganancia SIMD medida de 1,44×; acotar con `clamp/min/max` |
| SDR en tiempo real | carril real o complejo + lotes | matrices de convolución y de canal como literales (las constantes se pliegan) |
| EIS / electroquímica | carril complejo (`j` + `(f64,f64)`) | la fase es un compromiso físico; use lotes para barridos; **escriba el bucle caliente a mano** |
| FEM / elementos finitos | operadores matriciales `det/trace/inv` + jacobiano | `det` hasta orden 4, `inv` hasta orden 3; órdenes superiores vía biblioteca numérica |
| sistemas lineales de tamaño dinámico | carril en tiempo de ejecución (decls `_matrix`/`_vector`) | `solve(A,b)` está comprobado (rcond + residuo); devuelve `Result` |
| exploración dimensional | `himari-eval --groups` | grupos Π de Buckingham: sin derivar a mano el número de Reynolds |

> Todo lo que este repositorio **emite** — comentarios de doc del código generado, cargas de `compile_error!`, diagnósticos `FORMULA-ERROR` de la proc-macro, el encabezado WGSL — está en inglés. La única excepción que queda son los *títulos* de corrección rápida del LSP (sus diagnósticos están localizados); son una cadena de UI, no una ruta del catálogo de errores. La deuda de puesta en espera del catálogo es cero, y la puerta staging-budget de arriba (doce familias de construcción, condicionales y referencias `map_err` que producen cadenas incluidas, techo aplicado contra la base de fusión) es lo que la mantiene ahí.

**Errores con caret**, **sugerencias para nombres de función mal escritos** (`closest_func`, distancia de edición ≤ 2),
**rechazo de caracteres invisibles**, **límites de profundidad/pasos** — no son guarnición, son el listón para «atreverse a dejar que un colega use esto».

## Estado

El repositorio vive en la organización (`celestia-island/himari`, master único con squash). Las insignias de crates.io / docs.rs son entradas reservadas.
Las crates de biblioteca son **publicables**: `cargo publish -p himari --dry-run` empaqueta, reconstruye el archivo desde sus propias fuentes y llega al paso de subida, y los nueve nombres de crates siguen libres en crates.io.
Lo que queda es la subida misma — necesita un token de registro y es irreversible (un yank nunca libera un número de versión), así que es una decisión deliberada y separada, no un paso de build.
Los ejemplos, fixtures y las crates de herramientas permanecen no publicables.
Las dependencias internas ya llevan `version = "0.1"` con rutas locales, así que la subida debe seguir el orden topológico:
`himari → himari-latex / himari-matlab → himari-loader → himari-buildhelper → himari-macros`.
La publicación de las herramientas `himari-check` se decide por separado. `himari-standards` y `himari-lean` dependen de `himari-macros` y `himari-loader`, así que no cambian ese orden.
Historial de la hoja de ruta y los compromisos: `_reports/math-formula-20261122/` (REPORT / SPIKE-S0 / SYMBOLS-AND-PAPER).

## Verificación

```bash
cargo test --workspace --offline
cargo test -p himari-check --features wgsl-compile --test wgsl_compile --offline
cargo run -p himari-check --release --offline
python3 scripts/fmt_check.py
cargo test -p himari-example-common --offline
```

Las puertas cubren el álgebra del núcleo, la compilación real del código generado y los invariantes numéricos, ejemplos de consumidores escritos de forma independiente, la compilación real por naga
de WGSL, y la deriva del canónico/archivos auxiliares/bloqueo y del código generado. La cuenta exacta de pruebas es la que diga la salida actual de `cargo test --workspace`;
las cifras de rendimiento solo describen el entorno de bench de este repositorio y no son una garantía multiplataforma.

## No reivindicado

Estos son límites de alcance, no una lista de tareas casi terminada. Diga «no reivindicado» donde dice «no reivindicado».

- **Sin pivoteo en tiempo de ejecución en el carril simbólico**: el `solve` simbólico elimina en el orden de las expresiones y confía en `a ≠ 0`; no reordena filas por
  magnitud de pivote. Los sistemas mal condicionados o sensibles al pivoteo pertenecen al carril en tiempo de ejecución (`solve(A,b)` de arriba) o a una biblioteca numérica.
- **Matrices de patrón solo por el carril de archivos**: el carril de constructor para matrices de patrón necesita un nombre de archivo y un generador, así que las macros en línea las rechazan
  con una indicación; no hay expansión automática de matrices de patrón arbitrarias (nivel de investigación — todo lo que está fuera de las formas integradas se rechaza explícitamente
  con la lista soportada).
- **La validación numérica de WGSL está medida, pero en un dispositivo de software**: `just gpu-verify` compila el kernel emitido a SPIR-V, lo ejecuta a través de Vulkan y compara los resultados con el carril f64 del CPU (cuatro fórmulas, dieciséis puntos, peor error relativo 3.2e-7 en Mesa lavapipe; intercambiar un solo operador hace fallar el carril). Es una ruta de driver real ejecutando SPIR-V real, así que atrapa indexación, constantes y precedencia incorrectas — pero **no** mide el redondeo f32 específico del vendedor, la planificación de workgroups ni los bugs de driver, y no se ha hecho ninguna ejecución en GPU de hardware.
- **La verificación con Lean no ha tenido éxito**: no hay ninguna cadena de herramientas instalada en este host, así que solo se ejecutaron las pruebas de procesos simulados. No hay reivindicación de que Lean acepte
  las obligaciones de demostración generadas.
- **La publicación es una decisión separada e irreversible**: las crates de biblioteca son publicables y el dry-run está limpio, pero no se ha subido nada; la primera subida es el punto de no retorno para un número de versión. Véase Estado.
- **Alcance `no_std`**: `himari`, `himari-latex` y `himari-matlab` son `no_std + alloc`; las crates loader/buildhelper/macros/check no lo son.
  El código generado conserva su propio carril libm, y las fórmulas complejas no forman parte del carril `no_std`.
- **Sin derivadas analíticas complejas, sin variante f32 para fórmulas complejas o con valor matricial, sin derivada analítica para fórmulas con valor matricial.**
- **`roots` solo cubre los grados 1–2** en el lenguaje; los grados superiores solo mediante `himari-eval --roots` numéricamente.
- LaTeX general (macros/catcodes/composición) — semántica indecidible; la composición pertenece a la capa de renderizado.
- Sin inferencia de semántica a partir de glifos, y sin multiplicación `xy` en la superficie MATLAB. Sin `\sin^2 x`, sin `.*`, sin `$…$` (notación
  ambigua o puramente tipográfica: el análisis y el renderizado son dos trabajos distintos y un solo analizador no debería hacer ambos).
- Sin JIT en tiempo de ejecución en rutas de tiempo real/críticas para la seguridad; sin dependencias AGPL, nunca (evalexpr ≥12 y compañía quedan fuera).
- Los módulos **`himari-standards`** son calculadoras a nivel de cláusula, **no reivindicaciones de conformidad**: no modelan la geometría de la probeta, la selección del nivel de fuerza,
  el tiempo de permanencia, los presupuestos de incertidumbre, ni las propias reglas de informe de las normas, y no son una biblioteca general de unidades.
- El carril **`himari-lean`** demuestra una pequeña obligación entera cerrada (un ejemplo de determinante 3×3); no es un demostrador de teoremas general, y su puerta
  de axiomas es un analizador sobre la propia salida de Lean, no un argumento de solidez.
