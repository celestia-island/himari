<!-- markdownlint-disable MD033 MD041 -->
<!-- The logo lives in celestia-island.github.io/res/logos/himari.webp; that asset does not
exist yet, and this repository does not keep a copy of org assets. Add the <img> line here
once the logo lands, then drop this comment. -->
<h1 align="center">Himari</h1>

<p align="center"><strong>Une seule source de vérité, trois surfaces, compilées à Rust.</strong></p>

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
**Français** ·
[Español](./docs/es/README.md) ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

</div>
<!-- markdownlint-enable MD033 MD041 -->
**Ressources de formules : une seule source de vérité, trois surfaces, compilées en Rust.**

> **NOTICE : Ce logiciel contient du code généré par intelligence artificielle. Voir le fichier LICENSE pour les termes de la Synthetic Source License, y compris les exigences de divulgation des modèles.**
> Chaque ligne de source de ce dépôt a été générée par des modèles d'IA sous direction humaine ; pas une seule ligne n'a été écrite à la main. Les modèles impliqués sont listés à la fin de [LICENSE.txt](LICENSE.txt).
> Chaque capacité revendiquée dans ce README est étayée par des tests de ce dépôt et a passé six tours de revue contradictoire indépendante — mais **une revue par IA ne remplace pas une revue humaine**,
> et au moment de la rédaction, le contenu mathématique n'a pas été relu par un mathématicien.
> Le nom vient de Himari Akayama (Millennium Science School · Veritas) de Blue Archive —
> transformer des idées en machines qui tournent. Ni le nom sur crates.io ni le nom du dépôt de l'organisation n'entrent en collision avec quoi que ce soit (vérifié le 2026-11-23).

## De quoi s'agit-il

- Vous écrivez **une** formule (sous-ensemble de LaTeX / notation d'article / style MATLAB, au choix) ; elle descend vers le même
  **IR dimensionné**, puis se compile en une fonction Rust avec des intermédiaires nommés et des commentaires de source.
- Les dimensions (les sept grandeurs de base kg·m·s·A·K·mol·cd) sont imposées à la compilation : `m + s` ne compile tout simplement pas.
- La différentiation symbolique, l'élimination des sous-expressions communes (CSE) et le repli des constantes sont intégrés ; les dérivées partielles analytiques sont épinglées par des tests comparés aux différences centrées.
- **Les matrices sont des objets de première classe** : les matrices concrètes sont entièrement vérifiées ; les **matrices de motif** (avec des ellipses) expriment des tailles dynamiques —
  le `[b c ⋯ 0; a b c ⋯; ⋯ a b c; 0 ⋯ a b]` des manuels est du code source tel qu'il est écrit.

## Fédération et écriture multi-ligne (deuxième décision du 2026-11-23)

**Un seul bloc de macro peut contenir tout un groupe de formules qui se référencent mutuellement** (« fédération ») :

```rust
formula_matlab!{
    decl { F : force [N]; m : mass [kg]; r : radius [m]; }
    accel = F / m
    mech = ( accel * r )^(1/2)
}
// expands to a let chain: let accel = F / m; let mech = (accel * r).sqrt(); mech
```

- **Séparateur d'éléments** = point-virgule ou **retour à la ligne**, et cela **vaut dans les trois voies** (la voie des jetons reconstruit
  les retours à la ligne à partir des numéros de ligne des spans proc-macro, et ces numéros sont alignés sur le fichier source, si bien que les diagnostics pointent la même ligne).
- **Les éléments suivants peuvent référencer les précédents** : les dimensions se propagent avec le nom (le m/s² d'accel rejoint la table des symboles) ; la voie build.rs intègre la substitution au moment de la génération.
- **Le nom est le contrat** : quand une formule réutilise un nom de `decl`, la dimension de son résultat doit correspondre à cette déclaration
  (`a = F/m` convient, `s : Pa; s = F/d` est rejeté) ; les noms de formules en double au sein d'un même ensemble de formules ou d'un même répertoire de génération sont toujours rejetés.
- **Modèle de continuation des opérateurs** : les opérateurs doivent se trouver en **fin de ligne** (`x +⏎ y` ✓) ; un `-`/`+` en tête de la ligne suivante commence le signe d'un nouvel élément, ce n'est pas une soustraction continuée sur la ligne — c'est la décision de désambiguïsation.
- La multiplication implicite (juxtaposition) sur la surface LaTeX **doit rester sur une seule ligne** : juxtaposer sur plusieurs lignes avale la formule suivante comme facteur multiplicatif.

**Écriture de matrices sur plusieurs lignes** (une ligne physique par ligne de matrice ; l'espacement des colonnes est libre, aucun alignement requis) :

```text
[ 2*k, -k,    ...,   0 ;
  -k,  2*k,   -k,    ... ;
  ..., -k,    2*k,   -k ;
  0,   ...,   -k,    2*k ]
```

- Séparateur de lignes = `;` ou retour à la ligne (mélangeables, éventuellement en fin de ligne) ; séparateur de colonnes = virgule ou espace (≥1, n'importe quel nombre).
- Une ellipse `...` / `⋯` / `…` dans une cellule signifie « omis à cette position » (horizontal/vertical/diagonal est déduit lors de la réécriture).
- **Un `-` écrit sans espace devant est un signe, un espace le précédant commence une nouvelle colonne** (sémantique MATLAB) : `[k-1]` est une colonne, `[k -1]` en est deux.

## Nombres complexes : de bout en bout, de la couche de valeurs à la génération de code (2026-11-23)

- **`j` est l'unité imaginaire réservée** (toujours sans dimension, jamais déclarée, **jamais dans la liste des paramètres**). `Z = R_s + j*X` s'écrit directement.
- La couche de valeurs a exactement deux variantes : `Value::Scalar(C)` / `Value::Matrix(Vec<Vec<C>>)`, avec `C = {re, im}`.
  Un réel est un complexe de partie imaginaire nulle ⇒ **les matrices complexes, la transposition, la multiplication, l'addition et la soustraction sont
  toutes vraies élément par élément gratuitement** ; il n'y a pas besoin de scinder en quadrant « matrice réelle / matrice complexe ».
- Les fonctions transcendantales partagent une même table pour le réel et le complexe : `exp ln sqrt sin cos tan sinh cosh tanh` +
  `abs arg conj re im` (les quatre dernières retombent toujours sur l'axe réel). Les puissances complexes passent par `exp(w·ln z)` sur la branche principale.
- **`eval` signale une erreur explicite sur les valeurs complexes** (il ne supprime pas discrètement la partie imaginaire) ; utilisez `eval_value` dès que vous voulez des valeurs complexes.
- **La génération de code a une voie complexe séparée** : quand une formule contient `j`, elle est émise en `-> (f64, f64)` = (réel, imaginaire),
  et parmi les auxiliaires locaux au bloc (`__c_mul` / `__c_sqrt` …) **seuls ceux réellement utilisés sont émis** (sinon clippy `-D warnings` passe au rouge).
  Le point d'entrée par lots est `out: &mut [(f64, f64)]`, directement utilisable pour les balayages en fréquence EIS.
  La voie des macros (`formula!` etc.) trie de la même manière : `j` ne **se** dégrade **pas** en variable libre sur le site d'appel ;
  une liaison déjà complexe à l'intérieur d'une chaîne `let` fédérée est héritée correctement (`emit_block_complex_with`).
  La voie complexe **ne génère pas de dérivées analytiques** : `abs/arg/conj/re/im` ne sont pas analytiques complexes, donc la règle de la chaîne ne tient pas.
- **Le code généré et la couche de valeurs doivent concorder point par point** est une contrainte stricte : `__c_powf` copie le tri par exposant entier de `C::pow_real`
  (une puissance entière d'une base réelle négative doit atterrir exactement sur l'axe réel), et `__c_sqrt` utilise `hypot` comme la couche de valeurs pour éviter le débordement aux grandes magnitudes ;
  que `__c_div` suive IEEE alors que la couche de valeurs signale une erreur est délibéré — chacun de ces points est gardé par un test de comparaison par opérateur dans les fixtures.
- Les chaînes d'unités acceptent les exposants rationnels : `[V/A*s^(-1/2)]` (les crochets sont obligatoires — sinon le `/` de l'exposant ne peut pas être distingué du
  signe de division de la chaîne d'unités) ; un exposant qui ne se divise pas exactement (`m^(1/3)`) est rejeté explicitement. Écrivez `[1]` pour une grandeur sans dimension
  (gain / compte / fraction molaire doivent tous être déclarables ; ils ne peuvent pas s'introduire en douce en « omettant l'unité »).
- **`himari-fmt --diff <file>`** : aperçu en lecture seule qui imprime un diff unifié `--- current / +++ canonical`
  (aucun changement → aucune sortie et code de sortie 0 ; changements → code de sortie 1).
- **La frontière sémantique entre réel et complexe** (délibérée, pas un oubli) : `1/0` donnait `inf` et désormais **signale une erreur** ;
  `ln(0)` de même ; `(-2)^0.5` signale « le résultat est complexe, utilisez `eval_value` ».
  Le code généré suit en revanche la sémantique IEEE (division par zéro donne inf/NaN) : le chemin chaud ne porte pas de chemin d'erreur,
  et tout ce que la couche de formules peut prouver faux statiquement est intercepté par les gardes de dimension et de forme.
- **`python3 scripts/fmt_check.py --fix`** : une seule commande pour refaire la mise en page canonique et régénérer les fichiers d'accompagnement.

## Unités : préfixes SI, conversion, groupes sans dimension (2026-11-23)

- **Cherchez le nom complet avant de découper un préfixe** : `m` est le mètre (pas « milli-rien »), `mol` est la mole, `cd` est la candela,
  `daN` est le décanewton (le préfixe le plus long gagne). `nF` / `GHz` / `mm^2` / `mV/mm` / `µF` / `Ω` fonctionnent tous.
  Les unités dérivées couvrent les six que l'électronique ne peut éviter : `F` `Ω` (ou `ohm`) `S` `H` `Wb` `T`.
- **Dimension et facteur de conversion sont stockés séparément** : `parse_unit` ne donne que la dimension ; `parse_unit_spec` donne en plus le facteur de conversion →SI ;
  `convert(v, "nF", "F") = 1e-9 F`, et une incohérence de dimension provoque une erreur. L'unité de base de la masse est le **kg** :
  `g = 1e-3 kg`, `mg = 1e-6 kg`, `Mg = 1000 kg` ; les préfixes se comptent à partir du gramme (épinglé par un test de régression).
  **Les paramètres des fonctions générées sont toujours en unités de base SI**, un préfixe dans la déclaration n'affecte que la lisibilité — le commentaire de doc du code généré
  explicite le facteur de conversion pour chaque paramètre (cité verbatim d'un vrai commentaire de doc généré :
  `⚠ 参数 C_dl 声明为 µF：入参是 SI 基单位（1 µF = 0.000001 …）`).
- **Les préfixes CGPM 2022** figurent dans la table : `Q` (1e30), `R` (1e27), `r` (1e-27), `q` (1e-30), à côté des
  `da … Y / d … y` existants.
- **Unités non SI à définition exacte** (`unit_base` porte chaque facteur pour que `convert` reste dimensionnellement honnête) : `min`, `h`, `d`, `L`
  (ou `l` ; `mL` se décompose comme un préfixe sur `L`), `deg`, `arcmin`, `arcsec`, `bar`, `atm`, `eV`, `Wh`, `tonne`, `Da` (ou `u`), `nmi`, `knot`, `ha`, `acre`.
- **Les unités à décalage / logarithmiques sont rejetées explicitement** : `degC` (décalage) et `dBm` (un rapport) sont arrêtées avant le découpage du préfixe,
  en signalant la « vraie cause » plutôt qu'une « unité inconnue » — les traiter silencieusement comme des K ou comme sans dimension calcule une **valeur fausse**.
  **Les symboles ambigus sont rejetés de la même manière, sans devinement** : `cal` / `Cal` diffèrent entre la calorie thermochimique (4.184 J) et la calorie IT (4.1868 J), si bien que les deux sont
  refusées avec l'indication d'écrire `J` ; `rpm` exige un axe angulaire (1 rpm = 2π/60 rad/s) alors que ce dispositif traite `rad` comme sans dimension, il est donc refusé
  avec l'indication d'écrire `2 * pi * f` avec une déclaration `[Hz]`.
- **Groupes sans dimension (Buckingham Π)** : `dimensionless_groups` élimine avec des **rationnels exacts** (`f64` dérive),
  et donne directement les combinaisons sans dimension d'une base à partir de « noms de variables + dimensions » ; le nombre = (nombre de variables) − rang (le théorème Π comme assertion).
  Le contrôle classique : `{rho, v, L, mu}` → 1 groupe `rho·v·L·mu^-1` (le nombre de Reynolds).
  `dim_solve` est l'utilisation inhomogène du même système : `{kg, m/s²} → N` donne les exposants `[1, 1]`,
  et une dimension qui ne peut pas être formée signale « impossible de la former » plus le nombre de dimensions manquantes.

## Dimensions : exposants demi-entiers (2026-11-23)

`Dim` stocke en interne en unités de **1/2** (`[i16;7]` contient le double de l'exposant vrai), si bien que les puissances fractionnaires et entières
passent par la même arithmétique entière : `√s = s^(1/2)`, `f^(-1/2)`, `Ω·s^(-1/2)` sont tous légaux, et l'affichage utilise la forme entre parenthèses
comme `s^(-7/2)`. L'impédance de Warburg revient donc à la forme des manuels `Z = σ·f^(-1/2)`,
sans besoin d'adimensionnement par une fréquence de référence. L'exposant de puissance de `dim_check` **replie d'abord les constantes**,
si bien que l'écriture naturelle `f ^ (1/2)` passe la garde sans appeler `simplify` au préalable.

## Familles de fonctions : extremum / clamp / opérateurs matriciels (2026-11-23)

La **source de vérité unique** pour l'arité des fonctions est `func_arity` (dans core) ; les quatre voies (contrôle d'arité / évaluation / dimension / émission / réécriture)
la consultent — auparavant, chaque voie portait son propre `match`, et c'est ainsi que « `arg` émis comme une méthode inexistante dans la voie réelle » a survécu à une version.

| famille | fonctions | remarques |
| --- | --- | --- |
| scalaire unaire | `exp ln log sqrt sin cos tan sinh cosh tanh` | même table pour le réel et le complexe |
| complexe | `abs arg conj re im` | `arg` est sans dimension ; dans la voie réelle, re/im/conj sont l'identité |
| extremum/clamp | `min(a,b)` `max(a,b)` `clamp(v,lo,hi)` | même forme qu'en Rust : `v.max(lo).min(hi)` ; les trois arguments doivent partager une même dimension |
| opérateurs matriciels | `det` `trace` `adjoint` `inv` | **n'acceptent que des littéraux matriciels concrets** ; `det`/`inv` jusqu'à l'ordre 4/3 (suffisant pour les éléments isoparamétriques 2D et 3D), `adjoint` = transposée conjuguée |

`det`/`inv` utilisent la **récurrence par cofacteurs** (`det` ≤ 4, `inv` ≤ 3) : suffisant pour les cartes isoparamétriques des triangles 2D et des tétraèdres 3D,
avec des vérifications numériques pour `J·J⁻¹ = I` et `det(J)=1` ; un inverse 4×4 nécessiterait 16 cofacteurs 3×3
(≈100 termes) et ferait exploser le code généré, il est donc rejeté explicitement avec l'indication d'utiliser une bibliothèque numérique. `det`/`trace` **se développent avant de se différentier** dans `diff` —
le développement est une expression différentiable ordinaire, donc l'`area = det(J)/2` de la FEM a à la fois une valeur et une dérivée analytique
(`∂area/∂x₂ = (y₃-y₁)/2`).
La sélection par morceaux dans `min/max/clamp` n'est pas exprimable dans l'IR (il n'y a pas d'opérateur de comparaison), donc **aucune dérivée n'est générée** ;
une ligne de commentaire dans le code généré explique pourquoi (il ne panique plus). Les extrema et les clamp suivent la règle NaN de `f64::min/max`
(ils peuvent retourner l'autre opérande) ; les appelants qui exigent la propagation de NaN doivent vérifier `is_finite` à la frontière.

## Formes intégrées en un coup d'œil

`identity` / `diag(d)` / `diag_v(d[i])` / `tridiag(a,b,c)` / `tridiag_v(b[i])` /
`bidiag_u(b,c)` / `bidiag_l(a,b)` / **`blockdiag2(p,q)` = diag(p·I₂, q·I₂)** /
**`banded(v, k)` = matrice bande symétrique de demi-largeur de bande k** (k=2 n'est sans ambiguïté qu'à partir de 5×5 ; à 4×4 elle dégénère en matrice dense).

Ces noms décrivent la **forme d'IR reconnue** ; ce ne sont pas des fonctions que l'on peut appeler depuis une formule. Le point d'entrée est le
croquis de matrice avec ellipses, et tout ce qui n'est pas reconnu est rejeté explicitement.

Sémantique de la diagonale par blocs : à l'intérieur d'un bloc = scalaire × identité (p sur la diagonale, hors diagonale à l'intérieur du bloc vide), à l'extérieur des blocs doit être vide
(`0` et `...` sont équivalents) ; avec une dimension totale impaire, la cellule centrale va au bloc en bas à droite.

## Performance de la voie complexe : le côté honnête (mesuré le 2026-11-23)

`bin/bench.rs` mesure désormais quatre voies. La voie complexe, Z_c (balayage en fréquence EIS), **est plus lente que l'écrit à la main** ; consigné tel quel :

| forme | ns/point | par rapport à l'écrit à la main |
| --- | --- | --- |
| spécialisation écrite à la main (`σ/√ω·e^{-jπ/4}`, sans branchement, vectorisable) | 2.3 | 1× |
| transcription de la même forme (la séquence d'opérateurs du code généré recopiée dans une seule boucle) | 2.9 | 1.3× |
| code généré · appel scalaire direct | 6.4 | 2.8× |
| code généré · point d'entrée par lots | 12.8 | 5.5× |

Décomposition : **le √/÷ complexe générique coûte environ 2,2 fois plus que la spécialisation écrite à la main** (1 sqrt contre 3 sqrt + une division),
et **le wrapper par lots coûte encore environ 2,5 fois plus** (écritures dans des tranches de tuples + branchements empêchent la boucle de se vectoriser).

Optimisations déjà effectuées (chacune a réellement réduit l'écart) :

1. `__c_sqrt` est passé de la forme polaire (atan2+cos+sin, 4 fonctions transcendantales) à une **forme close algébrique** ;
1. chemin rapide purement imaginaire `√(iy) = √(|y|/2)(1+sign·j)` (un seul sqrt — la forme quotidienne en EIS) ;
1. `__c_div` utilise la forme naïve aux magnitudes ordinaires (1 division) ;
1. les auxiliaires complexes sont passés de **fermetures** à des `#[inline(always)] fn` locaux au bloc (15.5 → 11.9 ns/point).

**Conclusion (écrite dans le README plutôt que cachée)** : la voie complexe convient à « calcule correctement + assez vite », et ne vaut pas la peine de
courir après la limite d'une spécialisation écrite à la main ; pour la boucle vraiment chaude d'un balayage EIS, écrivez la spécialisation à la main et gardez la formule dans le
fichier `.formula` comme **source de vérité et contrôle de dimension**. La voie réelle n'a pas ce problème (scalaire 1,11×, au niveau de l'écrit à la main).

## Points d'entrée par lots : la longueur fixe gagne, les tranches perdent (mesuré)

Le générateur émet **deux** points d'entrée par lots par formule :

| forme | signature | mesuré (BV, 100k points) |
| --- | --- | --- |
| version tranche | `bv__batch(p0: &[f64], …, out: &mut [f64])` | 24.1 ns/point = **0,82× du scalaire (22 % plus lent)** |
| version longueur fixe | `bv__batch_n<const N: usize>(p: &[f64; N], …, out: &mut [f64; N])` | 19.3 ns/point = **1,03× du scalaire (3 % plus rapide)** |

Conclusion : **si vous voulez la performance, mettez la longueur dans un paramètre de type**. Les huit contrôles de bornes par point de la version tranche se trouvent dans la boucle et
LLVM ne peut pas tous les éliminer ; calculer la longueur commune une fois (une chaîne `.min()`) ou passer à des itérateurs zip n'atteint que 0,83×.
La version tranche est conservée comme forme commode (données de longueur variable) ; utilisez la version longueur fixe sur les chemins chauds.
(source du bench : `fixtures/three/src/bin/bench.rs` ; les deux pièges de méthodologie que nous avons rencontrés — construire des vecteurs dans la région chronométrée, et des
vecteurs constants laissant le chemin scalaire être replié à la compilation — sont consignés dans les commentaires de la source.)

## Algèbre matricielle (forme + évaluation + génération)

Les dimensions et les formes sont **deux axes de vérification orthogonaux** : les dimensions gouvernent le sens physique, les formes gouvernent la légalité algébrique.

| capacité | comportement |
| --- | --- |
| `shape_of` | matrice concrète → `(rows, cols)` ; la transposition échange ; matrice symbolique/de motif → `None` (pas d'inférence dynamique) |
| `shape_check` | l'addition/soustraction de matrices exige des formes égales, le produit matriciel exige `left cols = right rows`, les matrices irrégulières sont rejetées, la division matricielle est rejetée explicitement |
| `eval_value` | évaluation unifiée des scalaires/matrices, produit matriciel `C_ij = Σ_k A_ik·B_kj` |
| `flatten_matrix_value` | replie une expression matricielle **statiquement développable** en un IR de matrice concrète (transposition/addition/soustraction/négation/diffusion scalaire/produit matriciel) |
| émission | une matrice repliée → un littéral constant `vec![vec![…], …]` ; les grandes matrices passent par la voie matrice de motif + boucle |

```text
[1, 2; 3, 4] * [5, 6; 7, 8]  →  [[19.0, 22.0], [43.0, 50.0]]
[1, 2, 3]'                   →  3×1 column vector (emitted as vec![vec![1.0], vec![2.0], vec![3.0]])
A * [1, 2]                   →  scalar broadcast (a bare symbol is a scalar: matrices only come from literals/pattern generators)
[1, 2, 3] * [1, 2, 3]        →  compile-time rejection: matrix multiply impossible: (1, 3) × (1, 3)
```

> **Un `A,b` générique à l'exécution doit toujours être déclaré comme tel** : les noms de `decl` avec une grandeur se terminant par `_matrix`/`_vector` (ou égale à `matrix`/`vector`)
> deviennent des paramètres matrice/vecteur, ce que la voie d'exécution ci-dessous utilise. Tout *autre* symbole nu est un scalaire, donc `A * x` sur
> des matrices dont les formes ne font pas partie de la déclaration s'exprime encore par des littéraux, des expressions statiquement développables ou des générateurs de motifs.
> Pour un `Ax=b` général à grande échelle, l'inférence de taille à l'exécution, ou les systèmes mal conditionnés, utilisez une bibliothèque d'algèbre linéaire numérique.

**Puissances entières positives ou nulles de matrices** : `[a, b; c, d] ^ 2` est replié par le générateur en multiplications matricielles explicites (0 à 8 fois),
`M ^ 0` est la matrice identité de même ordre (le type de retour reste une matrice, pas le scalaire `1`). La vérification numérique de bout en bout est dans
`matrix_power.formula`. Un repli échoué (puissance négative/fractionnaire, hors plage, non carrée) émet `compile_error!`
plutôt qu'un accident de compilation comme `vec![…] * vec![…]` ; la couche de valeurs (`himari-eval`) donne le même résultat pour la même forme.

**Deux implémentations indépendantes doivent concorder** : la génération de code d'expressions matricielles (`flatten_matrix_value` + émission de littéraux)
et l'évaluation par la couche de valeurs (`eval_value`) sont écrites séparément, donc un **test différentiel** maintient les deux côtés alignés —
évaluer le littéral replié doit être égal à évaluer l'expression non repliée cellule par cellule (`mat-expr-fold`).

**Un croquis de matrice sans ellipses est une matrice concrète** : il passe par la voie des littéraux (les constantes se replient, pas de boucle d'allocation),
ce qui bat le générateur ; pour utiliser la voie du générateur (où `n` est un paramètre d'exécution), il faut écrire les ellipses.

## La voie matrice/vecteur d'exécution (2026-11-23)

Tout ce qui précède est statique : les formes sont connues à la compilation de la formule. La voie d'exécution est la réponse séparée pour
**les paramètres dont les tailles n'existent qu'à l'exécution** — un solveur consommant des données utilisateur, un assemblage FEM, tout ce qui reçoit `A` et `b` comme valeurs.

- **La déclaration est le seul interrupteur** : un `decl` dont la grandeur se termine par `_matrix`/`_vector` (ou est égale à `matrix`/`vector`) devient un
  paramètre matrice/vecteur — `A : stiffness_matrix [N/m]` arrive en `&[Vec<f64>]`, `b : load_vector [N]` en `&[f64]`.
  Les noms qui n'y adhèrent jamais restent des scalaires, donc les formules existantes ne sont pas affectées (`Decl::runtime_kind`).
- **Les fonctions générées peuvent échouer** : elles retournent `Result<_, himari::runtime::NumericError>`, un type d'erreur structuré unique
  (`InvalidInput` / `ShapeError` / `NonFiniteValue` / `SingularMatrix` / `IllConditioned` / `UnstableResidual` / `NoRealRoots`). Il n'y a
  **pas de contrat NaN silencieux** sur cette voie.
- **`solve(A,b)`** est une LU avec pivot partiel, puis deux gardes indépendantes avant qu'une réponse ne soit retournée :
  une **estimation de rcond** (`1/(‖A‖∞·‖A⁻¹‖∞)`, exigeant `rcond ≥ f64::EPSILON·64·n`) et un **contrôle du résidu relatif**
  (`≤ 1e-9`). L'inverse n'est formé que pour l'estimation du conditionnement, jamais comme moyen d'obtenir `x`. Un pivot singulier, une
  matrice mal conditionnée et une solution à grand résidu sont trois erreurs distinctes, pas un seul « échec ».
- **Également supportés sur cette voie** : `inv`, `det` (arguments littéraux seulement — un `det` à l'exécution est rejeté car sa dimension exige un ordre de matrice
  statique), `A*x`, `dot`, `norm`, et l'indexation avec `A(i, j)` en indices **basés sur 1** (`[I,J]` ; hors plage et zéro sont des erreurs, pas des panics).
- **Ce qu'elle émet** : le corps est une expression vérifiée sur des chaînes `let` en portée à l'intérieur de la fonction générée, et le type de retour suit le
  genre de valeur (`Result<f64, _>`, `Result<Vec<f64>, _>`, `Result<Vec<Vec<f64>>, _>`). Les formules de la voie d'exécution n'émettent **aucun** Jacobien analytique, **aucune**
  variante `__f32` et **aucun** point d'entrée par lots (ceux-ci exigent une signature scalaire de taille fixe).
- **Les réponses sont vérifiées face à des références indépendantes** : `runtime_matrix.formula` vérifie `solve(A,b)` face à une élimination de Gauss à pivotement
  distincte, `A*x` face à `b`, `A·inv(A)` face aux colonnes de l'identité, l'indexation basée sur 1, et un système à epsilon délibérément minuscule
  (`[[1e-17, 1], [1, 1]]`) que seul le pivotement résout correctement — le tout avec des tolérances de 2e-12/2e-15.
- **Voie des macros** : `formula_matlab!` accepte aussi les formules d'exécution, mais seulement **une formule par bloc** (une chaîne d'exécution reste sur la voie build.rs
  avec un message explicite) ; les paramètres matriciels complexes à l'exécution sont refusés pour l'instant.

## Qualité de la génération de code (2026-11-23)

- **Les puissances entières émettent `powi`** : `x^3` → `(x).powi(3)` (auparavant, elle émettait une méthode interne inexistante,
  si bien que tout `x^3` échouait à compiler avec l'erreur pointant dans le code généré). La fixture a une **garde de compilation** pour `x^3`.
- **Les fonctions inconnues échouent à la compilation** : les fonctions de la voie réelle sont une liste blanche ; `arg(x)` n'a pas de sens sur les réels →
  `compile_error!` ; `re/im/conj` sont l'identité sur les réels (exact, pas approché).
  Les appels à plusieurs arguments sont rejetés à l'analyse (l'émetteur ne regardait que `args[0]`, et abandonner silencieusement le reste calcule une valeur fausse).
- **Jacobien du système** : chaque fichier `.formula` produit en plus `{filename}__jacobian(all params)`,
  lignes = **formules scalaires réelles qui participent à la différentiation** (les éléments à valeur matricielle / complexes / non différentiables sont ignorés), colonnes = paramètres (ordre de déclaration),
  et une ligne qui ne dépend pas d'un paramètre est **0** dans cette colonne partout. C'est la forme dont ont besoin les solveurs implicites / les itérations de Newton / l'assemblage FEM.
- **Les constantes non finies échouent durement** : émettre `inf` / `NaN` produirait un identifiant illégal,
  donc il panique avant la génération en nommant l'origine (`contains_nan_expr` est la source de vérité unique).
- **Parenthèses autour des puissances** : `(x^2)^3` et `x^(2^3)` reçoivent des parenthèses sur les deux surfaces —
  `x ^ 2 ^ 3` se réanalyse en `(x^2)^3` (x⁸ → x⁶, c'est-à-dire que le formatage canonique **changerait les mathématiques**).

## Formatage (canonique)

`himari-fmt` : `cargo run -p himari-check --bin himari-fmt -- [--check|--write] <file>`.

- Decls multi-lignes (un symbole par ligne) ; une formule par ligne ; **colonnes de matrice alignées** (chaque colonne = contenu le plus large + ≥1 espace, la virgule comme frontière de colonne).
- Les lignes de commentaire autonomes remontent toutes en tête de fichier (ordre préservé) — les commentaires sont pour les humains et n'entrent jamais dans l'IR.
- `--check` = la garde de dérive (câblée dans `just fmt-check` / la CI) ; la sortie canonique est idempotente en aller-retour.
- **Le canonique garde les virgules** : une séparation par espace seul et une soustraction binaire sont indiscernables à la réécriture (`-k` serait avalé comme terme de soustraction),
  si bien que la virgule est la frontière non ambiguë ; écrire à la main sans virgules est acceptable, le formatage les remet.

## Voies (décision de l'utilisateur du 2026-11-23)

| voie | usage | adaptée à |
| --- | --- | --- |
| **macro en ligne** | `formula_matlab!{ decl { a : l [m]; b : l [m]; } a / b }` | formules simples |
| **macro à chaîne** | `formula_latex!{ r"decl { … } \frac{a}{b}" }` | formules simples (surface LaTeX) |
| **macro fichier externe** | `formula_file!("formulas/bv.formula", matlab)` | formules complexes (le dialecte doit être donné explicitement) |
| **build.rs** | voir `packages/check/fixtures/three/` | code généré versionné + garde de dérive (la voie formelle) |

Le `decl` à l'intérieur d'une macro ne fournit que des informations de symboles pour le contrôle de dimension ; il **ne crée pas de variables locales Rust** : `a/b` au
site d'appel doit déjà être `let a = …; let b = …;` par l'appelant. Pour un exemple compilable complet, voir
`inline_macro_example()` dans `examples/common/src/main.rs`.

> ⚠️ Limitation connue : une proc-macro qui lit des fichiers **n'est pas suivie par cargo** (`tracked_path` est encore sur nightly),
> donc après avoir modifié un `.formula`, il faut un `touch` pour déclencher une recompilation ; pour la production, utilisez la voie build.rs (`rerun-if-changed`).
> ⚠️ La **voie du constructeur** pour les matrices de motif/transpositions n'existe que pour les fichiers / build.rs (elle a besoin d'un nom de fichier et d'un générateur) ;
> les macros en ligne signalent une erreur explicite avec une indication quand elles les rencontrent, elles ne se dégradent jamais silencieusement.

## Langages de surface (trois, un seul IR)

| | article | LaTeX | style MATLAB |
| --- | --- | --- | --- |
| lettres grecques | `α·x` | `\alpha \cdot x` | `alpha*x` |
| ellipse | `⋯ …` | `\cdots \vdots \ddots` | `...` |
| matrice | `[a, ⋯, b; ⋮ ⋱ ⋮; c, ⋯, d]` | `\begin{bmatrix}…\end{bmatrix}` | `[a, ..., b; ...]` |
| transposée | `A'`, `A′`, `A^T` | `A^T` | identique à gauche |
| décls de dimension | `decl { v : velocity [m/s]; }` | identique à gauche | identique à gauche |

Règles (le rejet est une fonctionnalité) :

- Chaque symbole doit d'abord être déclaré avec sa dimension ; non déclaré = erreur de compilation, pas d'inférence à partir des glyphes.
- Une notation ambiguë ou purement typographique comme `\sin^2 x`, `\left(\right)`, `$…$`, `x .* y` est rejetée d'emblée.
- La multiplication implicite ne vaut que sur les surfaces LaTeX/article (le système à une seule lettre est non ambigu par construction) ; la surface MATLAB exige un `*` explicite.
- Les matrices de motif ne prennent en charge que trois formes prouvablement correctes : `identity(n)`, `diag(d, n)`,
  `tridiag(a, b, c, n)` ; tout ce qui n'est pas reconnu = erreur + la liste de ces trois.
- Les sources peuvent être manuscrites (marques combinantes, etc.), mais **l'IR doit être normalisé** — les caractères combinants n'entrent jamais dans l'IR.

## Algèbre vectorielle / Horner / no_std (2026-11-23)

**`dot(a,b)` / `norm(v)`** : développés uniquement sur des **littéraux concrets** (forme connue à la compilation) ;
la dimension de `dot` est le produit des deux côtés, `norm` a la même dimension que ses éléments. `dot([x,y],[x,y])` (x=3,y=4) → 25,
`norm` → 5. Des formes incompatibles sont rejetées d'emblée.

**Conversion automatique de Horner** : un **polynôme explicite** écrit comme `c₀ + c₁x + c₂x² + …` dans la source est converti avant l'émission en
`(…((cₙx + cₙ₋₁)x + …) + c₀)` — le nombre de multiplications passe de `2n-1` à `n`, et les grandes puissances comme `x^n` disparaissent
(numériquement plus stable). `a*x^3 + b*x` → `(a * x * x + b) * x`.
**Les formes compactes ne sont pas développées** : `(u-1)^3` reste tel qu'écrit (le développer ne fait que le rendre plus difficile à lire) ; les dérivées sont aussi prises sur
**l'original avant Horner** (différentier la forme de Horner produit une chaîne de produits qui s'annulent).

**Voie `no_std`** : `himari_buildhelper::generate_dir_libm` produit du code qui n'utilise que `core` + `libm`
(transcendantales → `libm::exp` etc., puissances entières → `libm::pow` ; `f64::powi` est une chose de std).
Cela signifie que le **code généré** est utilisable en `no_std` — et depuis le 2026-11-23, la pile de compilation l'est aussi :
`himari`, `himari-latex` et `himari-matlab` sont `no_std + alloc` derrière une fonctionnalité `std` par défaut (`std` est activée sauf si vous la désactivez).
Le chargeur, le buildhelper, les macros et les crates de vérification (E/S de fichiers, hôte proc-macro, outillage) restent en `std` et activent la fonctionnalité explicitement.
Le code généré conserve **sa propre voie libm**, contre laquelle un consommateur bare-metal se lie réellement.
Garde : une crate fixture `#![no_std]` (`[lib] test = false`, car le harnais de test a besoin de std) qui à la fois compile
du code généré **et** exerce la chaîne d'analyseurs (`himari_matlab::parse` → `dims_of_set` → `shapes_of_set` → `dim_check_runtime` → `format_set`)
sans std. **`cargo build` qui passe = le code généré et le frontal ne tendent pas secrètement vers std.**
Cibles bare-metal vérifiées : `riscv32imc-unknown-none-elf` (core seulement — pas d'atomiques disponibles), `thumbv7em-none-eabi`, et
`x86_64-unknown-none` (chaîne d'analyseurs). Cette voie n'émet pas de Jacobien (il faut `alloc`) ni de variante f32
(les transcendantales f32 sont la famille `*f` de libm, et nous refusons de livrer une demi-voie) ; pour les dérivées partielles, utilisez les fonctions `__d_*` par formule
(scalaire pur, allocation nulle).

## Bibliothèques de formules inter-fichiers : `use "..."` (2026-11-23)

```text
use "lib/atmosphere.formula"      % file header, may span several lines
decl { ... }
v_terminal = sqrt(2 * m * g / (rho * Cd * A))
```

- **L'analyse ne fait que collecter, l'E/S appartient au chargeur** : le frontal reste une API purement chaînes (`FormulaSet.uses`),
  et `himari-loader` se charge de la lecture des fichiers, de l'inclusion récursive, de la détection de cycles et de la résolution des conflits.
- Sémantique de fusion (simple et prévisible) : les decls/éléments référencés sont **placés d'abord** → la fédération fonctionne comme d'habitude (les éléments suivants peuvent référencer les précédents) ;
  les decls identiques en double sont dédupliqués, des orthographes différentes provoquent une erreur ; les noms d'éléments en double provoquent toujours une erreur (**pas de « le dernier écrit gagne »**) ;
  les cycles provoquent une erreur et impriment la chaîne de noms de fichiers autour du cycle.
- **Une bibliothèque est « substituée », pas « appelée »** : une fois `rho` développé chez l'appelant, la liste des paramètres contient `rho_0 / h / H`.
  C'est délibéré (aucun état caché, différentiable statiquement) ; le coût est une liste de paramètres plus longue.
- Convention de répertoire : `generate_dir` ne scanne que les `*.formula` de **premier niveau**, donc `formulas/lib/*.formula`
  n'existe naturellement que comme bibliothèque et n'est jamais généré deux fois comme formule de premier niveau.
- **Le dialecte vient de la macro, pas d'un devinement** (décision de l'utilisateur du 2026-11-24, « la macro est le dialecte ») : la macro qui enveloppe la formule
  décide quelle grammaire la lit.

  | macro | dialecte |
  | --- | --- |
  | `formula!(…)` | **matex** (surface canonique ; le nom du point d'entrée est conservé, la sémantique est fixée) |
  | `formula_latex!(…)` | LaTeX |
  | `formula_matlab!(…)` | matex (synonyme de `formula!`, plus explicite à la lecture) |
  | `formula_file!("f.formula", latex)` | décidé par le second argument **obligatoire** |

  `formula_file!` ne rend pas l'argument de dialecte optionnel : le contenu du fichier ne peut pas être inféré du nom de la macro, et faire deviner la macro
  ne ferait que cacher l'ambiguïté ailleurs.

  **Pourquoi ce n'est pas qu'une préférence de style** — cela élimine une vraie valeur fausse silencieuse. Quand `si` est déclaré, `si * x` :
  matex le lit `si·x`, LaTeX le lit `s·i·x`. Mesuré sur le même fichier avec les mêmes entrées (s=3, i=5, x=2, si=100) :

  ```text
  old AUTO (try matex first, use if it succeeds)  →  200    without reporting any error
  explicit LaTeX                                  →  30
  ```

  Les deux diffèrent de près d'un facteur sept. AUTO n'avait aucun moyen de savoir ce que l'auteur voulait dire — ce qu'il « devinait » n'était que quel analyseur
  réussissait en premier. C'est pourquoi l'énumération `Dialect` de la voie des macros n'a **pas** de variante `Auto` : le système de types garantit que le devinement ne peut pas revenir.

## Démarrage rapide : `examples/reentry` (un projet minimal exécutable)

```text
examples/reentry/
  formulas/entry.formula       % terminal velocity / dynamic pressure / drag (aerospace side)
  formulas/structures.formula  % static displacement = solve(k·x − F, x)
  build.rs                     % three lines: generate_dir("formulas", "src/generated.rs")
  src/main.rs                  % prints the table + physical invariant self-checks
  src/generated.rs             % the checked-in generated code (reviewable, covered by the drift gate)
```

`cargo run -p himari-example-reentry` imprime une table vitesse→pression dynamique→traînée et vérifie :

- **Invariant physique** : `drag(v_terminal) == m·g` (différence relative 1.6e-16 — ce n'est pas un ajustement, c'est la définition)
- **Dérivée analytique vs quotient de différences** : `∂drag/∂v` concorde à une différence relative de 1.2e-8 entre les deux chemins
- **Résolution symbolique** : `x_static` vient de `solve(k·x − F, x)`, et `∂x/∂F = 1/k` tient automatiquement
- **La variante f32** et le **Jacobien du système d'une formule incluse depuis une bibliothèque** sont tous deux dans le même fichier généré

Un consommateur n'a plus besoin que de `himari-buildhelper::generate_dir(dir, out)` — toute la voie build.rs
(analyse / fédération / dimensions / formes / dérivées / f32 / points d'entrée par lots / Jacobien / LaTeX source) se réduit à une seule fonction,
si bien que chaque projet ne recopie plus 300 lignes.

## Exemples communs : `examples/common` (génération réelle + références numériques indépendantes)

```bash
cargo run -p himari-example-common
cargo test -p himari-example-common
```

| fichier de formule | exemple | point d'entrée généré et vérification |
| --- | --- | --- |
| `matrices.formula` | rotation, transposition, `AᵀA`, inverse 2×2, `A·A⁻¹`, matrice fois colonne | `rotation_orthogonality` / `N` / `stiffness_inverse` / `nodal_forces` ; cellule par cellule face au calcul manuel |
| `springs.formula` | système d'équilibre de ressorts à trois nœuds `solve([…],[u₁,u₂,u₃])` | `displacements` face à une élimination de Gauss à pivot partiel Rust distincte |
| `runtime_matrix.formula` | `A`/`b` d'exécution : `solve(A,b)`, `A*x`, `inv(A)`, indexation basée sur 1 | `runtime_displacements` / `runtime_traction` / `runtime_inverse` / `runtime_element` ; chaque appel faillible (`Result`), pivotement prouvé sur `[[1e-17,1],[1,1]]` |
| `least_squares.formula` | équations normales d'un ajustement linéaire | `line_fit` face à la forme close directe des moindres carrés, donnant `y=1.04+1.99x` |
| `roots.formula` | le quadratique et la petite racine pour `b²≫4ac` | `quadratic_roots` vérifie `1,2` et `10⁻⁸,10⁸` (protection contre l'annulation) ; retourne `Result`, et un discriminant négatif est `NumericError::NoRealRoots` |
| `circuit.formula` | `I=V/(R+j·2πfL)` | `current` / `current_magnitude` vérifient `|I|=0.995037 A` ; `2π` est écrit explicitement |
| `dynamic_matrix.formula` | stencil tridiagonal dynamique `n×n` | `diffusion_stencil(k,n)` cellule par cellule face à une référence construite en boucle |
| macro en ligne (`src/main.rs`) | `formula_matlab!` avec `a/b`, et `solve(a, b)` sur des formes d'exécution | des variables locales Rust réellement passées ; `m/m=1` vérifié à la compilation ; la résolution d'exécution vérifiée numériquement |

Ces exemples prouvent que les **formes actuellement supportées** vont réellement du `.formula` à du Rust exécutable ; ils ne présentent pas
`solve` comme un solveur numérique général, et les exemples statiques n'exigent pas de paramètres matriciels d'exécution.
Le `src/generated.rs` généré est versionné et protégé par la garde de dérive.

## Voie des normes : `himari-standards` (2026-11-23)

`packages/standards` transforme une poignée de **normes d'ingénierie publiées** en calculateurs typés, une relation au niveau de l'article par module, chacune
portant sa propre `PROVENANCE` (identifiant de la norme, édition, l'article dont vient la formule, et ce qui n'est *pas* modélisé). Rien ici ne cite
la prose d'une norme, et rien ne revendique la conformité à la norme — cela reformule la formule comme une simple expression. La conversion d'unités est
délibérément absente : les appelants passent des unités de base SI.

| module | norme | édition | calculé |
| --- | --- | --- | --- |
| `iso_6506_1_2014` | ISO 6506-1 | 2014 | dureté Brinell `HBW` (plus la pression d'empreinte moyenne en Pa) |
| `iso_6892_1_2019` | ISO 6892-1 | 2019 | résistance à la traction `Rm`, allongement en pourcentage `A`, module `E` |
| `iso_2533_1975` | ISO 2533 | 1975 | troposphère `T`, `p`, `rho` et la conversion géopotentielle (0 … 11 000 m, et il refuse au-delà plutôt que d'extrapoler) |
| `gb_t_231_1_2018` | GB/T 231.1 | 2018 | dureté Brinell `HBW` ; édition courante, remplace GB/T 231.1-2009, modifie et adopte ISO 6506-1:2014 |
| `gb_t_228_1_2021` | GB/T 228.1 | 2021 | résistance à la traction `Rm`, allongement `A`, module `E` ; édition courante (en vigueur le 2022-07-01) |
| `gb_t_8170_2008` | GB/T 8170 | 2008 | arrondi décimal, **arrondi au pair**, décidé sur des chaînes décimales — `f64` n'intervient jamais dans une décision d'arrondi |

Les deux modules GB d'essais mécaniques réutilisent l'arithmétique ISO (même évaluation testée) mais gardent leur **propre** identifiant, édition et provenance, si bien qu'une
réponse ne peut jamais être étiquetée avec la mauvaise norme. Pour `GB/T 228.1-2021`, le degré d'adoption de tout document ISO n'a **pas** été vérifié
de façon indépendante, donc le module ne revendique explicitement aucune équivalence — seules les trois définitions sont implémentées. Les éditions ont été vérifiées contre la
Plateforme publique de service d'information sur les normes nationales, et chaque module consigne cette vérification dans ses notes de provenance.

L'outil CLI `himari-standard` évalue une requête et imprime un document de réponse à forme stable (JSON), en lisant la requête comme JSON ou TOML —
depuis `--input <path>` ou stdin, avec `--format json|toml` pour forcer l'analyseur et `--list` pour les identifiants et éditions exacts :

```bash
himari-standard --input packages/standards/examples/brinell.json
himari-standard --input packages/standards/examples/tensile.toml
himari-standard --list
```

Ces trois requêtes d'exemple sous `packages/standards/examples/` sont exercées par la suite de tests,
si bien que les invocations documentées ne peuvent pas pourrir.

Les clés inconnues sont refusées plutôt qu'ignorées ; chaque échec retourne le même document avec `values` à `null` et la cause dans `errors`
(code de sortie 1, avec un diagnostic d'une ligne sur stderr) ; le code 2 est réservé à un mauvais usage de la ligne de commande. Les requêtes dont les décimales participent à une **décision
d'arrondi** doivent porter ces nombres comme chaînes décimales, car un flottant binaire a déjà perdu le dernier chiffre au moment où la crate le voit.

## Voie de preuve : `himari-lean` (2026-11-23)

`packages/lean` est un **co-processus** Lean 4 : il génère une obligation de preuve autonome, la confie à un vrai exécutable `lean` via stdin
(`lean --stdin --json -T <heartbeats> -M <MB>`), et rapporte ce qui s'est passé dans un document structuré. Pas de projet Lake, pas d'index de paquets,
pas de réseau ; le fichier généré n'a aucun import.

- **Les données dynamiques ne deviennent jamais que des entiers littéraux** d'un *énoncé de théorème d'exemple nommé* — jamais une définition de données, un axiome ou un identifiant.
- **Vérifié par le noyau, par `decide`** : l'obligation est close sur `Int` et déchargée par `by decide`. Aucun `native_decide`, `#eval`,
  `implemented_by`, `extern`, `partial`, `sorry` ou axiome utilisateur n'est émis, et un test vérifie le texte généré contre cette liste.
- **Garde des axiomes** : la sortie de `#print axioms` est analysée et seuls `propext`, `Classical.choice` et `Quot.sound` sont tolérés ; un rapport illisible est une
  violation, jamais une réussite. L'absence de rapport n'est pas la preuve d'un rapport propre.
- **Un oracle indépendant** : le même déterminant est calculé en Rust avec un développement *différent*, donc Lean n'est jamais le seul témoin.
- **`lean_missing` est un non-succès de première classe** : cela signifie que la chaîne d'outils n'était pas là et que **rien n'a été vérifié**. Il n'est jamais rendu comme
  « vérifié » et ne sort jamais avec le code 0. Statuts et codes de sortie : `success` 0, `lean_missing` 3, `invalid_request` 4, `proof_failed` 5,
  `resource_exhausted` 6, `axiom_violation` 7, `oracle_mismatch` 8.

**Vérifié contre une vraie chaîne d'outils (Lean 4.36.0, commit `611c117b`).** L'obligation du jalon est acceptée par le noyau et ne rapporte aucun
axiome du tout :

```text
status = success            verified = true       exit = 0
lean_message: 'det3_example' does not depend on any axioms
oracle:       rust-i128-permutation det3([1,2,3,4,5,6,7,8,10]) = -3
```

Les contrôles négatifs se comportent comme prévu avec le même binaire : une requête qui affirme `expected = -306` retourne `oracle_mismatch` (code de sortie 8) et n'est
jamais rapportée comme vérifiée, et la suite à processus factices couvre les chemins `proof_failed` / `axiom_violation` / `resource_exhausted`.

Le provisionnement est externe à ce dépôt et reste une opération ponctuelle : installez `elan`, épinglez `leanprover/lean4:v4.36.0`, puis soit mettez `lean` sur le `PATH`
soit faites pointer `HIMARI_LEAN_BIN` dessus. `just lean-verify` émet l'obligation et exécute le test sur Lean réel. **Sans chaîne d'outils, la voie se dégrade en
`lean_missing` (un non-succès) et `--emit` écrit quand même un fichier `.lean` relisible** — l'absence de Lean n'est jamais rapportée comme une vérification.

## Robustesse et entrées adverses (2026-11-23)

- **Limite de profondeur 2048** : chaque sortie d'analyse vérifie une fois la profondeur de l'IR (`check_depth`), et tout ce qui dépasse la limite signale
  « formule imbriquée trop profondément (N levels > limit 2048) » au lieu de laisser une récursion en aval déborder la pile (SIGSEGV).
  La limite de pile de pest elle-même se déclenche aussi et son message est traduit en mots simples également. **Frontière connue** : une longue chaîne `a + b + c + …` est
  imbriquée à gauche dans l'IR, donc la profondeur égale le nombre de termes ; le nombre de termes d'une formule est donc plafonné à environ 2048.
  Une garde déterministe distincte contre l'imbrication de parenthèses (256 niveaux, `MAX_PARSE_NESTING`) s'exécute avant l'analyse, car elle doit aussi tenir là où la
  fonctionnalité optionnelle `stack-guard` de pest est indisponible (`no_std`).
- **Les caractères invisibles sont toujours rejetés** : espace de largeur nulle / override bidirectionnel / joiner de largeur nulle / BOM / NUL sont tous
  nommés et refusés avant l'analyse (`scan_source`) — ils peuvent faire qu'une formule « paraisse différente à la relecture de ce qu'elle est ».
- **Limite d'étapes de somme/intégrale 10^7** : `Σ(i,1,1e9)` passe de « tourne toute la nuit » à une ligne d'erreur.
- **Un appel de fonction avec des arguments manquants** ne panique plus par indexation `args[0]` : les trois voies (évaluation/émission/dimension) le rejettent explicitement.
- **Tests de propriétés structurés** (pas de la soupe de caractères aléatoire) : 600 expressions aléatoires passent chacune trois gardes — la réécriture canonique se réanalyse en
  **même arbre**, la réécriture LaTeX se réanalyse en **même arbre**, et `simplify` **préserve la valeur**.
  Les trois montrent actuellement 0 non-équivalence (`prop-roundtrip`) ; une grande entrée sommant 1500 termes se termine en moins de 100 ms.

## Voie simple précision (f32) (2026-11-23)

Chaque formule **scalaire réelle** produit en plus `{name}__f32(p: f32) -> f32` — la même formule,
les mêmes règles d'émission, seuls les littéraux passant au suffixe `f32` (`Fx::F32`, l'interrupteur unique traversant l'émetteur).
f32/f64 mesuré = **1,44×** (`SIMD` 8 voies contre 4 voies + la moitié de la bande passante),
dont les moteurs de jeu / le DSP audio / la SDR temps réel bénéficient directement.

Les formules complexes et à valeur matricielle ne produisent **pas** de variante f32 : le tuple `(f64, f64)` de la voie complexe et le
`Vec<Vec<f64>>` de la matrice sont un contrat de type différent en simple précision ; cela peut venir plus tard (nous refusons de livrer une demi-voie).

## Outils pour développeurs (2026-11-23)

Trois points d'entrée sont livrés ensemble (tous dans la crate `himari-check`, partageant un seul analyseur) :

| point d'entrée | but |
| --- | --- |
| `himari-fmt --diff` | n'afficher que la différence entre la forme canonique et la forme actuelle (pas de réécriture) |
| `himari-fmt --diagnose` | diagnostics d'analyse + dimension + forme uniquement (silencieux avec code de sortie 0 si tout est propre) |
| `himari-fmt --symbols` | lister les noms `decl` et les noms de formules (la source de données pour la complétion d'éditeur) |
| `himari-eval <file> --set k=v …` | **évaluer** : `Re = 200000 [1]` ; `--groups` imprime aussi les groupes Π de Buckingham ; `--json` pour les machines |
| `python3 scripts/fmt_check.py --fix` | une seule commande pour remettre en forme canonique + régénérer les fichiers d'accompagnement |

**Les erreurs portent des carets** : `FrontError::render_with_source` imprime la ligne de source et dessine un `^` sous la colonne fautive
(les trois voies — build.rs / fmt / eval — passent par lui) — plus besoin de compter jusqu'à la colonne 17 soi-même.

**L'extension VS Code** (`packages/vscode`) a deux canaux :

| canal | déclencheur | source de données |
| --- | --- | --- |
| CLI (par défaut, zéro installation) | ouverture / enregistrement | `himari-fmt --diagnose` + `--symbols` |
| **LSP** (optionnel) | **erreurs pendant la frappe** | `himari-lsp` (JSON-RPC sur stdio) |

`himari-lsp` est un serveur minimal écrit à la main (aucune dépendance à un cadre LSP) : `initialize` /
`didOpen` / `didChange` / `completion` / `codeAction` / `shutdown`, avec **synchronisation incrémentale du texte**.
Ses diagnostics portent le même `himari::Error` que la bibliothèque, rendus dans la langue choisie par `HIMARI_LANG` (repli sur `LC_ALL`, `LANG`, puis l'anglais) ; les neuf langues du projet sont disponibles.
La couche protocole traite l'état et les paramètres JSON-RPC comme des fonctions pures, et il y a un test de bout en bout qui démarre réellement un processus et envoie réellement des
trames `Content-Length`. Définissez `himari.lspPath` dans les réglages pour basculer (laissez vide = canal CLI).

Les candidats de complétion = les noms `decl` du fichier + les noms de formules + la table des fonctions unaires (y compris `det/inv/solve/dot/norm` etc.).

## Surface LaTeX : les noms multi-lettres exigent `\mathrm` (2026-11-23)

En mode mathématique LaTeX, un `sigma` nu est **cinq variables multipliées s·i·g·m·a** (multiplication implicite),
donc les noms multi-lettres de ce dialecte doivent s'écrire `\mathrm{sigma_0}` — et la réécriture les émet de la même manière.
Les noms de lettres grecques (`sigma`, `omega`…) sont réécrits comme les **glyphes réels** `\sigma`, en conservant l'indice
(`\sigma_0`). L'implémentation utilisait autrefois le remplacement de sous-chaînes, si bien que `alphabet` devenait `\alphabet` et
`betamax` devenait `b\etamax`, tandis que `sigma` s'affichait comme cinq lettres italiques — faux à trois endroits à la fois (survol / fichier d'accompagnement / source de vérité de la documentation). Corrigé (`latex_var`).

## Erreurs : une seule énumération, neuf langues

Chaque échec du crate est une seule énumération, `himari::Error`. Ce n'est pas un sac de chaînes :
chaque échec est une variante qui **porte ses données**, ce qui donne trois choses à la fois.

- **Un code stable** — `err.code()` renvoie un identifiant en kebab-case (`dimension-mismatch`,
  `undeclared-symbol`, `no-real-roots`, ...). Les tests et les appelants vérifient le code, donc un
  changement de formulation ou une traduction ne peut jamais les casser.
- **Un rendu par langue** — `err.render(locale)` et `Display` (anglais) produisent la phrase à partir
  du catalogue de [i18n.rs](packages/himari/src/i18n.rs). `HIMARI_LANG` / `LC_ALL` / `LANG`
  sélectionnent la langue aux bords CLI et LSP ; les neuf locales sont celles de l'organisation
  (`en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `fr`, `es`, `ru`, `ar`).
- **Compatibilité `anyhow`** — `Error` implémente `core::error::Error` (donc `std::error::Error`
  sous `std`) et est `Send + Sync + 'static`, sans aucun emballage :

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

`tests/anyhow_compat.rs` verrouille tout cela : la propagation par `?`, la borne
`Send + Sync + 'static`, le fait que `downcast_ref::<himari::Error>()` retrouve le code après
`anyhow::Context`, et la règle selon laquelle la même erreur se rend différemment dans les neuf
langues tandis que le code reste identique.

`Error::Message(String)` subsiste pour les messages qui n'entrent jamais dans le catalogue ; il s'affiche tel quel dans toutes les langues. La dette de mise en attente qu'il représente est **mesurée par une garde**, pas par de la prose — et cette dette est actuellement nulle :

```text
python3 scripts/staging_budget.py                # counts + ceiling
python3 scripts/staging_budget.py --against-git  # also: the ceiling may only go down vs the merge base
```

C'est un parcours d'AST (`tools/staging-budget`, fondé sur `syn`) : le compte ne dépend ni de la disposition des lignes, ni des blancs, ni des commentaires, et un *motif* tel que `matches!(e, Error::Message(m) if …)` n'est jamais pris pour une construction. Douze seaux répartis en trois familles sont comptés dans chaque arbre de production de l'espace de travail (`packages/`, `examples/`, `tools/`) : les six formes de construction littérales (`Error::from(`, `Error::Message(`, et dans les frontaux `FrontError::plain("…")` / `plain(format!(…))` / `FrontError::new(…, "…")`), les entonnoirs de blanchiment de valeurs (`plain` / `new` avec une charge utile à valeur d'exécution, et `FrontError::plain` passé comme référence de fonction, tous deux dans tout le dépôt), et les formes de mise en attente de chaînes (`ok_or`/`ok_or_else`/`map_err` produisant des chaînes en attente, et `Err(format!(…))`) à l'intérieur du crate du catalogue lui-même. Les éléments compilés seulement sous les tests et le catalogue sont ignorés ; une variante cataloguée construite avec une charge utile de chaîne (p. ex. `Error::SyntaxError(text)`) n'est pas de la mise en attente et n'est pas comptée. La garde échoue si le total dépasse le plafond enregistré (le plafond vit dans `tools/staging-budget/src/ceiling.txt` et ne peut que baisser ; il est actuellement à `0`). Le plafond ne peut que baisser, et c'est appliqué : la CI (et `just verify`) exécute la garde avec `--against-git`, qui lit `tools/staging-budget/src/ceiling.txt` à la **base de fusion avec le tronc distant** et échoue si le plafond de l'arbre de travail est plus élevé — de nouveaux sites ne peuvent donc pas se glisser dans une hausse du plafond. La commande nue ci-dessus ne vérifie que le compte. Les 51 variantes du catalogue sont épinglées par `catalogue_completeness_tests`, qui construit les 50 variantes du catalogue plus la `Message` de mise en attente et les rend dans les neuf langues (non vide, aucun `{n}` qui fuit). Les *titres* de correction rapide du LSP sont encore en chinois ; ses diagnostics sont localisés.

### Intégration éditeur (LSP)

`himari-lsp` est un service LSP écrit à la main (la couche protocole est en fonctions pures, avec des tests unitaires + un test stdio de bout en bout) :

| capacité | description |
| --- | --- |
| diagnostics | pousse `publishDiagnostics` après `didOpen`/`didChange` (symboles non déclarés, incohérences de dimension, erreurs de syntaxe) |
| complétion | les noms `decl` du fichier + les noms de formules + la table des fonctions unaires (`det/inv/solve/roots/...`) |
| **correction rapide** | `textDocument/codeAction` : symbole non déclaré → ① réorthographier vers le nom déclaré le plus proche ② ajouter une ligne au bloc decl ; nom de fonction mal orthographié → le nom issu du registre |
| synchronisation | **synchronisation incrémentale** (`textDocumentSync: 2`, découpée par plage) ; les changements complets sont aussi acceptés, donc les anciens clients ne cassent pas |

Une correction rapide ne propose un remplacement que lorsque le nom apparaît **exactement une fois** (s'il apparaît plusieurs fois, elle ne propose que « ajouter la déclaration »,
sans jamais deviner laquelle) ; les valeurs hors plage sont toujours **ramenées dans les bornes** plutôt que de perdre le changement (en perdre un
désynchroniserait définitivement le tampon de l'éditeur).

### Verrous de contenu des bibliothèques de formules : quand une bibliothèque change, elle doit le dire

`use "lib/…"` **inclut** le fichier de bibliothèque, donc quand la bibliothèque change, le code généré en aval change avec elle — exactement le genre de
régression la plus facile à manquer. `--lock` écrit une empreinte de chaque fichier référencé dans `<entry>.lock`, et à partir de là **le verrou est vérifié à chaque chargement** :

```text
$ himari-fmt --lock formulas/entry.formula
已写入 formulas/entry.formula.lock（4 行）

$ himari-eval formulas/entry.formula         # someone quietly changed the library
himari-eval: 公式库与锁文件不一致（formulas/entry.formula.lock）：
  内容变了（声明版本 1.4.2 → 2.0.0）：lib/atmosphere.formula
              复核改动后运行 `himari-fmt --lock <入口>` 重新生成锁。
```

(La sortie du CLI est citée verbatim : l'outil imprime encore en chinois. En anglais : la
bibliothèque et son fichier de verrou sont en désaccord — `lib/atmosphere.formula` a changé sa version déclarée 1.4.2 → 2.0.0 — relancez
`himari-fmt --lock <entry>` après avoir revu le changement.)

- **Hachage de contenu sha256** (pas une plage semver) : pour une forme de dépendance comme « source incluse », un hachage de contenu est la
  garantie la plus forte — tout changement est attrapé, aucune politique de résolution de version n'est nécessaire, aucune règle de pré-version.
- **Texte brut, déterministe, chemins relatifs au répertoire de l'entrée** : il peut aller dans git, le diff est lisible, et il est comparable entre machines.
- **Garde fermée** : `scripts/fmt_check.py` lance `--lock-check` sur les fichiers qui ont un `.lock`
  (et reverrouille automatiquement sous `--fix`) — un verrou périmé passe au rouge dans la garde, sans attendre que quelqu'un charge le fichier.
- **opt-in** : sans `.lock`, rien n'est vérifié (impact nul sur les dépôts existants). Une fois un verrou existant, **une nouvelle référence échoue aussi**
  (sémantique de liste blanche) ; reverrouillez pour l'accepter.
- **Un verrou illisible est aussi un échec** : pas de l'UTF-8 (enregistré en UTF-16 par un éditeur), permissions insuffisantes, ou un chemin de répertoire
  sont tous des erreurs dures — ils ne peuvent jamais être traités comme « pas de verrou », ce qui désactiverait silencieusement la protection.
- **`--fix` a trois étapes** (tout formater d'abord → puis tout reverrouiller → enfin régénérer chaque fichier d'accompagnement) :
  les fichiers d'accompagnement vérifient le verrou au chargement, donc le verrou doit être final d'abord ; les deux dépendent aussi du contenu final des bibliothèques référencées,
  et les mélanger dans une seule boucle laisse des artefacts périmés.
- `%! version: 1.2.0` n'est que la colonne lisible par l'humain (elle est verrouillée et un changement de version est signalé) ; la garantie sémantique repose sur le hachage.
- Pourquoi pas de plages semver : cela nécessiterait toute une politique de résolution/épinglage/pré-version, alors que la forme de dépendance de ce dépôt est
  « source incluse ». Pour exprimer « autoriser une dérive mineure », le reverrouillage est l'énoncé le plus direct.

### Directives de dialecte : le pendant de « la macro est le dialecte » pour la voie fichier

Un fichier n'a pas de macro pour l'envelopper, donc la règle correspondante est : **matex par défaut ; déclarer LaTeX explicitement.** Les deux voies partagent un principe —
pas de devinement, c'est l'auteur (pas l'analyseur) qui décide de la sémantique.

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

- Prend en charge `%! dialect: latex` (ou `tex`) et `%! dialect: matex` (ou `matlab`).
- **Pas de directive = matex**. Il y avait autrefois ici un repli AUTO (si l'analyse matex échouait et que la source contenait `\`, essayer LaTeX),
  et il produisait exactement la valeur fausse silencieuse 200-contre-30 ci-dessus — il a été supprimé.
- Une notation purement LaTeX sans directive de dialecte → analysée comme le matex par défaut → **erreur** (jamais une valeur fausse silencieuse) ;
  l'erreur offre une issue (écrire `%! dialect: latex`) mais **ne devine pas à votre place**.
- Elle ne prend effet qu'**avant la première ligne non commentaire** (un `%!` dans des commentaires du corps ne change pas le dialecte).
- Une valeur non reconnue est une erreur.
- Avec une directive, les erreurs sont signalées dans ce dialecte : une erreur de syntaxe sous une directive LaTeX est une erreur LaTeX.
- C'est implémenté à exactement un endroit (`himari_loader::parse_source`), donc la voie build.rs, `himari-fmt`,
  `himari-eval` et les diagnostics LSP concordent tous. (les diagnostics LSP sont rendus dans la langue choisie par `HIMARI_LANG` ; voir la section « Erreurs ».)

### Division sur la surface LaTeX (autrefois absente)

`a / b` et `a \div b` sont désormais une vraie division, synonyme de `\frac{a}{b}`, à la même priorité que la multiplication et associative à gauche.
Auparavant, la surface LaTeX n'avait **aucun opérateur de division** — seul `\frac{a}{b}` était accepté, et `a / b` était une erreur de syntaxe.
C'était jeter le bébé avec l'eau du bain : `/` est la façon la plus ordinaire d'écrire une division et il n'y avait aucune raison de la refuser.
(Le repli AUTO ci-dessus avait en fait couvert ce défaut : il faisait « marcher par hasard » `a / b` tout en apportant la valeur fausse silencieuse.
Corrigez le défaut et le repli n'a plus de raison d'exister.)

## Résolution linéaire symbolique `solve` (2026-11-23)

La forme à une variable `solve(equation, variable)` résout `equation = 0` en `variable = -b/a` (l'équation doit être **linéaire** en cette variable).

```text
ES_ss = solve(k_on * E * S - (k_off + k_cat) * ES, ES)
      → ES_ss = k_on·E·S/(k_off + k_cat)          % Michaelis–Menten steady state
```

- La solution **est encore de l'IR** : elle conserve donc le contrôle de dimension, l'évaluation, la génération de code, les dérivées analytiques et
  le Jacobien — aucun second mécanisme n'est nécessaire. Les dérivées partielles et les lignes de Jacobien de `ES_ss` sortent automatiquement.
- **La liste des paramètres = les variables que l'expression émise utilise réellement** : le `ES` résolu n'apparaît pas dans les paramètres de `ES_ss`
  (« le demander mais devoir le passer » est corrigé). Le fichier d'accompagnement / le survol / la complétion lisent la même liste de paramètres.
- **La non-linéarité est rejetée explicitement** : `x·x`, `1/x`, `x^2`, `sin(x)` signalent une erreur **à la sortie de l'analyse** et nomment le genre concerné ;
  aucune approximation numérique, aucune réponse douteuse. Se tromper sur un équilibre chimique coûte bien plus que de taper quelques caractères de plus.

### La forme système `solve([equations…], [unknowns…])`

Quand les deux arguments sont des listes, la voie du **système linéaire** est entrée, et la solution est un **vecteur colonne** (la voie de valeurs d'expression matricielle,
retournant `Vec<Vec<f64>>`) :

```text
x = solve([(k1 + k2) * x1 - k2 * x2 - F1,
           -k2 * x1 + (k2 + k3) * x2 - F2], [x1, x2])
  → x = 2×1 column vector, dimension [m]           % two-mass spring statics
```

- **Élimination symbolique** : l'élimination de Gauss–Jordan s'exécute sur l'**anneau des expressions** (les coefficients sont des sous-expressions IR, pas des nombres),
  simplifiée en forme normale à chaque étape. Chaque composante résolue est encore de l'IR et passe par les dimensions/l'évaluation/l'émission comme d'habitude.
  **Mais une solution de système a une valeur matricielle** : les formules à valeur matricielle ne génèrent pas de dérivées analytiques (les dérivées partielles élément par élément ne sont pas une matrice ; le Jacobien l'est),
  et ne génèrent pas de variante f32 — c'est la limite existante de la voie à valeur matricielle, pas spécifique à `solve`.
- **Convention de forme** : les listes d'équations et d'inconnues sont **aplaties** en ordre ligne-majeur, donc `[e1, e2]` et `[e1; e2]` sont équivalents —
  les utilisateurs n'ont pas à se souvenir d'une orientation. Les entrées de la liste d'inconnues doivent être des noms de variables nus.
- **Les inconnues ont besoin de dimensions déclarées** : ce que cela rapporte est le **contrôle de la dimension du résultat** (l'exemple des ressorts résout `m = N/(N/m)`),
  ainsi que le contrôle d'**homogénéité** des équations elles-mêmes (`x1 − F1`, ajouter m à N, est rejeté à l'analyse).
  Les inconnues résolues n'entrent pas dans la liste des paramètres (la fonction générée ne prend que des paramètres réels).
- **Le rejet est une fonctionnalité** : sous-déterminé (pas de rang plein), incohérent (après élimination `0 = nonzero`), non linéaire (l'inconnue apparaît des deux côtés d'une
  multiplication / dans un dénominateur ou un exposant / à l'intérieur d'un argument de fonction) sont **tous signalés à la sortie de l'analyse**.
  Les rejets liés aux dimensions se produisent dans `dims_of_set` / l'étape de génération (l'analyseur lui-même ne fait aucun contrôle de dimension).
  Les équations redondantes sont supprimées « au sens repliable » : une ligne redondante à coefficients symboliques qui ne se replie pas à zéro est, de façon conservatrice,
  signalée comme incohérente — mieux vaut refuser que retourner une mauvaise solution.
- **Pivot non nul et frontière numérique** : même convention que `solve` à une variable, ce qui ne peut pas être prouvé nul est traité comme
  non nul (nous faisons confiance à `a ≠ 0`). L'élimination a lieu à la compilation dans l'ordre des expressions et **ne réordonne pas les lignes par |pivot| à l'exécution** ;
  aux échelles extrêmes la forme close s'annule, un paramètre singulier signale une division par zéro dans la couche de valeurs et donne `inf/NaN` selon IEEE dans
  le code généré — ce n'est pas une exception. Les systèmes mal conditionnés, ou qui nécessitent une stratégie de pivotement, doivent aller à une bibliothèque numérique.
- **Frontières (quatre, toutes rejetées à la sortie de l'analyse ou comme erreurs de compilation)** :

  1. Un coefficient ou terme constant **à valeur matricielle** (`solve(M*x - b, x)`, `solve([M*x - b], [x])`) → rejeté explicitement avec une
     indication vers `inv(A) * b` — sinon la voie scalaire générerait une division `vec!`, qui ne compile pas.
  1. **Les équations peuvent avoir des dimensions différentes** (un équilibre de forces en N plus une contrainte géométrique en m est un système légal), mais chaque équation doit être
     homogène de son côté ; et chaque composante de la solution doit partager **une seule dimension** (des inconnues à dimensions mêlées — résoudre à la fois pour un courant et une
     résistance — ne sont pas encore supportées ; c'est une limite de la voie à valeur matricielle).
  1. La voie complexe (avec `j`) fonctionne dans **la couche de valeurs** et **la couche d'émission** (terminée le 2026-11-23) : un vecteur colonne complexe est
     émis en `Vec<Vec<(f64, f64)>>`, les éléments étant (réel, imaginaire). L'analyse nodale de circuits AC et les « systèmes d'équations complexes » de style EIS
     peuvent donc être générés directement en code.
  1. **Le test de zéro sur les lignes redondantes est une condition suffisante** : les zéros que la combinaison des termes semblables peut prouver (`k−k`, `k·a−a·k`, `x·x²−x³`)
     sont reconnus ; une identité complexe qui ne peut pas être prouvée nulle est traitée comme un coefficient non nul (conservateur — elle peut signaler à tort
     sous-déterminé/incohérent, mais elle ne retournera pas une mauvaise solution).

### Racines de polynômes `roots(poly, x)`

`solve` donne une forme close linéaire ; `roots` donne **toutes les racines d'un polynôme** (sous forme de vecteur colonne) :

```text
roots_quad = roots(a * x ^ 2 + b * x + c, x)
% q = −(b + sign(b)·√(b²−4ac))/2; large root = q/a, small root = c/q
% Return order still matches the original formula (−b−√Δ first); it does not depend on sorting the roots by size.
```

- **Seulement degré 1–2** : le degré 1 est la même mathématique que `solve` ; le degré 2 est la formule close des racines.
  **Le degré 3 et au-delà est rejeté explicitement** avec le degré réel signalé — Cardano/Ferrari sont terriblement mal conditionnées en virgule flottante,
  et ce dépôt ne distribue pas une approximation « qui a l'air précise ». Pour les racines de degré trois et plus, calculez d'abord les coefficients numériquement
  puis utilisez une bibliothèque numérique.
- **Les racines multiples sont répétées selon leur multiplicité** (`x² − 2x + 1` donne deux 1), conformément au théorème fondamental de l'algèbre ;
  le repli `q=0` pour une racine nulle est `-b/(2a)` et ne passe pas par `0/0`.
- **Éviter l'annulation dans la formule quadratique** : utiliser `(−b±√Δ)/(2a)` directement perd la petite racine quand `b²≫4ac`
  (`x²−10⁸x+1` calculait jadis `10⁻⁸` comme environ `7.45×10⁻⁹`). On utilise maintenant les stables `q/a` et `c/q`, et la ligne d'acceptation pour l'erreur relative de la petite racine
  dans le code généré est `1e-12` ; les deux directions de signe et la voie complexe ont des régressions.
- **Les dimensions sortent automatiquement** : l'équation doit être dimensionnellement homogène, et l'homogénéité implique directement « la dimension d'une racine = la dimension de la
  variable », donc aucune dérivation supplémentaire de dimension n'est nécessaire (un test de contrat garde ce raisonnement).
- **Discriminant négatif dans la voie réelle** : le code généré retourne une erreur typée, **pas NaN** — le point d'entrée quadratique réel `roots` retourne désormais
  `Result<_, himari::runtime::NumericError>` et un discriminant négatif est `NumericError::NoRealRoots { discriminant }`. Il n'y a plus de contrat NaN
  silencieux, donc les contrôles `is_nan`/`is_finite` ne sont plus la façon de détecter ce cas ; un discriminant constant
  prouvablement négatif à la compilation est toujours rejeté d'emblée. `eval_value` est la couche générale de valeurs complexes et **retourne encore des racines complexes**
  pour la même expression dans la voie complexe ; une voie complexe a besoin d'un `j` explicite.
- **Les termes d'ordre supérieur qui s'annulent sont retirés** : `x³ − x³ + x − 1` est traité comme de degré 1 (pas « rejeté parce qu'un x³ apparaît dans le polynôme »).
- **Les non-polynômes sont rejetés explicitement** : `x` à l'intérieur d'un argument de fonction, dans un dénominateur, ou à une puissance non entière — chacun avec sa raison donnée.
- **Les coefficients à valeur matricielle** sont rejetés (même convention que `solve`, sinon la voie scalaire générerait une division `vec!`).

### Vecteur de coefficients `poly_coeffs(poly, x)`

Le complément de `roots` : il donne les **coefficients** (du degré bas vers le haut), retournant un vecteur ligne `1×k`.

```text
coeffs = poly_coeffs(2 * s ^ 2 - 3 * s + 1, s)   → [[1, −3, 2]]
```

- **La variable doit être sans dimension**. La dimension du coefficient de degré k d'un polynôme homogène est `dim(poly)/dim(x)^k`, et ils diffèrent ;
  mais une matrice exige des **éléments d'une seule dimension** — prendre les coefficients d'une variable dimensionnée est donc algébriquement impossible sous cette forme.
  Ce contrôle l'explique directement au stade du diagnostic et offre une issue (diviser par une grandeur caractéristique, ou utiliser `roots`) plutôt que de laisser
  l'aval signaler « les dimensions des éléments de matrice diffèrent ».
- Les coefficients nuls de degré élevé sont retirés (`x³ − x³ − 2x + 1` est de degré 1) ; le polynôme nul est rejeté.
- Les non-polynômes (`sin(x)`, `1/x`, puissances non entières) sont rejetés.
- Il partage le même extracteur `poly_coeffs` que `roots` — une seule définition de la sémantique des coefficients, deux utilisateurs.

### Racines au-delà du degré trois : `himari-eval --roots`

Rejeter le degré ≥ 3 dans le langage est un compromis forcé, mais la recherche de racines **numérique** fonctionne pour tout degré, donc elle vit dans la couche outil :

```text
$ himari-eval cubic.formula --roots
cubic = 1, 2, 3  [1] (degree 3, 3 roots)
cplx  = -1, 0.5-0.8660254037844387j, 0.5+0.8660254037844386j  [1] (degree 3, 3 roots)
```

- **Itération de Durand–Kerner** (réutilisant le `C` du cœur pour l'arithmétique complexe, sans bibliothèque numérique tierce) : les valeurs initiales sont les puissances de
  `0.4 + 0.9j`, le rayon est la **moyenne géométrique des magnitudes des racines** `|c₀/cₙ|^(1/n)` (repli sur la
  borne de Cauchy `1+max|cᵢ|` seulement dans les cas dégénérés) ; au plus 1000 itérations ; le critère DK est `|Δz| ≤ 1e-10·radius` (relatif à la magnitude des racines,
  pas relatif à 1) ; suivi de **60 étapes de polissage de Newton** (critère d'arrêt relatif `1e-15·|z|`).
- **La non-convergence est une erreur**, jamais une approximation pas-encore-convergée retournée comme résultat ; `inf`/`NaN` pendant l'itération est aussi une erreur immédiate
  (`f64::max` retourne l'autre côté pour NaN, ce qui a jadis fait qu'une étape NaN ressemblait à « convergé » et retournait 20 NaN).
- **Acceptation par le résidu** : après itération, l'erreur inverse relative `|p(z)| / Σ|aₖ|·|z|ᵏ` doit être suffisamment petite (seuil 1e-8),
  sinon c'est une erreur — DK ne garantit pas la convergence vers une racine, et le critère doit être indépendant de l'itération elle-même.
  Le dénominateur **doit inclure le terme `k=n`** : sans lui le dénominateur dégénère en la constante `|a_n|` aux petits `|z|`,
  et `x²−1e-100` passerait tout en étant faux d'un facteur 4e25 (une vraie régression).
- **Polissage de Newton** (critère relatif) : une racine simple atteint la précision machine ; **une racine multiple ne converge que linéairement**, donc `x²−2x+1` et
  ses semblables ont une erreur relative de l'ordre de `1e-9` et ne signalent pas d'erreur — c'est une propriété de l'algorithme, pas un bug ; ne traitez pas une racine multiple comme
  une racine simple.
- **Limite de degré 16** (limite de développement de polynôme de `poly_coeffs` ; `numeric_roots` lui-même supporte jusqu'à 64).
- **Normalisation monique** avant itération (division par le coefficient dominant) pour éviter le débordement de magnitude ; limite de degré 64 ; coefficients `inf`/`NaN` rejetés.
- **La mise à zéro du bruit efface seulement la partie imaginaire, et le critère est relatif** : une racine réelle porte un résidu imaginaire d'échelle `1e-16`, qui est un
  artefact d'itération ; le critère est `|im| ≤ 1e-12·|z|`. **La partie réelle n'est jamais touchée** (les racines de `x²−1e-50` sont ±1e-25),
  et **un seuil absolu ne convient pas non plus** — pour une racine complexe dont la magnitude est inférieure à 1e-12, la partie imaginaire est toute l'information de la racine
  (`x³−1e-60` a jadis été étalé en trois racines réelles sur cette base, erreur relative 0,87, sans erreur).
- Les coefficients doivent s'évaluer en nombres sous les paramètres donnés par `--set` ; quand la variable est omise, elle est inférée comme « le seul symbole déclaré de la
  formule », et plusieurs candidats exigent `--var`.
- La sortie porte le degré et le nombre de racines ; les racines complexes s'écrivent `a+bj`.

## Matrices complexes et systèmes d'équations complexes (2026-11-23)

La complexité n'est pas seulement scalaire : les éléments de matrice et les vecteurs de solution peuvent aussi être complexes, et **la couche de valeurs et la couche d'émission concordent**.
Le type de retour de la fonction générée est `Vec<Vec<(f64, f64)>>` (élément = (réel, imaginaire)) :

```rust
// I = solve([Z*I1 - V, Z*I1 - R*I2], [I1, I2]) with Z = R + jωL
pub fn I(R: f64, w: f64, L: f64, V: f64) -> Vec<Vec<(f64, f64)>> { … }
```

- Les usages typiques sont l'**analyse nodale de circuits AC** et la modélisation EIS : les inconnues sont des courants/potentiels complexes.
- La transposition est une **transposition simple** (pas de conjugaison) — la conjugaison est l'opérateur distinct `conj` ; ne les confondez pas.
- `adjoint` est la transposée conjuguée ; seuls les littéraux dont la forme est connue à la compilation peuvent être développés.
- Dans la voie complexe, `det/trace/adjoint/inv/solve/roots/dot/norm` **de même** se développent au niveau de l'IR avant l'émission (asymétrie historique :
  la voie réelle développait tandis que la voie complexe allait droit à `compile_error!`).
- Toujours non supportés : la **voie no_std/libm** pour les formules complexes (les fermetures dépendent de std), les **dérivées analytiques** pour les formules complexes
  (`abs/arg/conj/re/im` ne sont pas analytiques complexes, cela nécessite le calcul de Wirtinger), et la **variante f32** pour les formules complexes.

## Voie WGSL (GPU) : sous-ensemble + frontière de validation (2026-11-23)

`himari-fmt --wgsl <file.formula>` émet une formule scalaire réelle sous forme de noyau f32 :

```wgsl
// Generated by himari (f32 scalar lane; really compiled and validated by naga)
fn q(rho: f32, v: f32) -> f32 {
    return 0.5f * rho * (v * v);
}
```

**Sous-ensemble supporté (volontairement petit)** : `+ - * /`, `sqrt`, `exp`/`log` (le `ln` de ce dépôt) `sin cos tan
sinh cosh tanh abs min max clamp`, plus les puissances 2/3, la racine carrée, et la puissance -1.
**Explicitement rejetés** : complexe (`j`), matrices, `det/inv/solve`, Σ/∫, opérateurs symboliques, autres puissances,
et les symboles entrant en collision avec les mots réservés WGSL — chacun avec une raison (nous n'émettons pas un noyau qui ne compile pas).

**La validation a deux couches** (depuis le 2026-11-24 la voie WGSL a une preuve **au niveau du compilateur**) :

1. **Vraie garde de compilation** (faisant autorité) : `cargo test -p himari-check --features wgsl-compile`
   confie chaque noyau émis dans les fixtures à **naga** (le compilateur de shaders de wgpu, l'une des implémentations faisant autorité de la spécification WGSL) pour
   une vraie analyse + une vraie validation. naga est une dépendance de développement purement CPU — **aucun GPU nécessaire**, et elle n'entre jamais dans l'arbre des dépendances d'exécution.
1. **Garde statique** (pré-filtrage rapide, `wgsl::validate_wgsl`) : équilibre des parenthèses, une signature `-> f32`, exactement un
   `return`, des littéraux flottants portant `f`, pas de `^`/`**`, pas de constantes non finies. Sa valeur est « attraper les erreurs grossières dans les environnements sans naga
   + des messages d'erreur plus proches de cette voie » ; **ce n'est pas une preuve de correction**.

**Pourquoi une vraie garde de compilation est obligatoire — une leçon concrète** : chaque noyau de cette voie WGSL était autrefois **invalide**.
WGSL n'a pas de fonctions à corps expression ; le corps doit être `return expr;`, alors que l'émetteur émettait toujours une expression nue ;
pire, la garde statique **interdisait `return`** — parce que la règle avait été écrite sans connaître cette partie de la spécification, transformant le
malentendu en « la règle ». L'angle mort structurel d'une garde statique n'est pas « elle va rater des choses » mais « **elle ne sait pas ce qu'elle a manqué,
et elle peut même juger du code correct comme faux** ». La première exécution de naga a exposé toute la classe.
Autre croyance corrigée : `return 2.5;` est **légal** dans naga (le flottant abstrait atterrit en f32 par le contexte) — donc « les flottants doivent porter `f` » est une convention de style
de cette voie, pas la spécification WGSL.

**Frontière de validation (énoncée honnêtement)** : naga ne valide que **la syntaxe et les types**. « Le noyau calcule-t-il les bons nombres sur un vrai GPU » reste
non vérifié — cela nécessite un environnement d'exécution wgpu + un GPU (node-3 a une RTX 5880, connexion en attente).
De plus, tint (l'implémentation de Chrome/Dawn) et naga diffèrent sur quelques règles de bord ; ce dépôt n'est responsable que devant naga.

## Choisir une voie selon le domaine (un seul tableau)

| scénario | voie suggérée | remarques |
| --- | --- | --- |
| HPC scientifique à grande échelle / bibliothèques numériques | `.formula` + build.rs, scalaires réels | au niveau de l'écrit à la main (1,0×) ; la variante par lots à longueur fixe supprime les contrôles de bornes |
| moteurs de jeu / DSP audio | idem + **`{name}__f32`** | gain SIMD mesuré de 1,44× ; serrer avec `clamp/min/max` |
| SDR temps réel | voie réelle ou complexe + lots | matrices de convolution/canal en littéraux (les constantes se replient) |
| EIS / électrochimie | voie complexe (`j` + `(f64,f64)`) | la phase est un engagement physique ; utilisez les lots pour les balayages ; **écrivez la boucle chaude à la main** |
| FEM / éléments finis | opérateurs matriciels `det/trace/inv` + Jacobien | `det` jusqu'à l'ordre 4, `inv` jusqu'à l'ordre 3 ; ordres supérieurs via une bibliothèque numérique |
| systèmes linéaires de taille dynamique | voie d'exécution (decls `_matrix`/`_vector`) | `solve(A,b)` est vérifié (rcond + résidu) ; retourne `Result` |
| exploration dimensionnelle | `himari-eval --groups` | groupes Π de Buckingham : plus besoin de dériver le nombre de Reynolds à la main |

> Tout ce que ce dépôt **émet** — commentaires de doc du code généré, charges utiles de `compile_error!`, diagnostics `FORMULA-ERROR` de la proc-macro, en-tête WGSL — est en anglais. La seule exception restante est constituée des *titres* de correction rapide du LSP (ses diagnostics sont localisés) ; ce sont des chaînes d'interface, pas un chemin du catalogue d'erreurs. La dette de mise en attente du catalogue est nulle, et la garde staging-budget ci-dessus (douze familles de construction, conditionnels et références `map_err` produisant des chaînes inclus, plafond appliqué par rapport à la base de fusion) est ce qui l'y maintient.

**Erreurs avec carets**, **suggestions pour les noms de fonctions mal orthographiés** (`closest_func`, distance d'édition ≤ 2),
**rejet des caractères invisibles**, **limites de profondeur/d'étapes** — ce ne sont pas des ornements, c'est la barre pour « oser laisser un collègue l'utiliser ».

## État

Le dépôt vit sur l'organisation (`celestia-island/himari`, master unique en squash). Les badges crates.io / docs.rs sont des entrées réservées.
Les crates de bibliothèque sont **publiables** : `cargo publish -p himari --dry-run` empaquette, reconstruit l'archive depuis ses propres sources et atteint l'étape de téléversement, et les neuf noms de crates sont encore libres sur crates.io.
Ce qui reste, c'est le téléversement lui-même — il exige un token de registre et il est irréversible (un yank ne libère jamais un numéro de version), donc c'est une décision délibérée et séparée plutôt qu'une étape de build.
Les exemples, fixtures et crates d'outillage restent non publiables.
Les dépendances internes portent déjà `version = "0.1"` avec des chemins locaux, donc le téléversement doit suivre l'ordre topologique :
`himari → himari-latex / himari-matlab → himari-loader → himari-buildhelper → himari-macros`.
La publication de l'outillage `himari-check` est décidée séparément. `himari-standards` et `himari-lean` dépendent de `himari-macros` et `himari-loader`, donc elles ne changent pas cet ordre.
Historique de la feuille de route et des compromis : `_reports/math-formula-20261122/` (REPORT / SPIKE-S0 / SYMBOLS-AND-PAPER).

## Vérification

```bash
cargo test --workspace --offline
cargo test -p himari-check --features wgsl-compile --test wgsl_compile --offline
cargo run -p himari-check --release --offline
python3 scripts/fmt_check.py
cargo test -p himari-example-common --offline
```

Les gardes couvrent l'algèbre du cœur, la compilation réelle du code généré et les invariants numériques, des exemples de consommateurs écrits indépendamment, la compilation réelle par naga du
WGSL, et la dérive du canonique/des fichiers d'accompagnement/du verrou et du code généré. Le nombre exact de tests est ce que dit la sortie actuelle de `cargo test --workspace` ;
les chiffres de performance ne décrivent que l'environnement de bench de ce dépôt et ne sont pas une garantie multiplateforme.

## Non revendiqué

Ce sont des limites de périmètre, pas une liste de tâches presque terminée. Dites « non revendiqué » là où il est écrit « non revendiqué ».

- **Pas de pivotement à l'exécution dans la voie symbolique** : le `solve` symbolique élimine dans l'ordre des expressions et fait confiance à `a ≠ 0` ; il ne réordonne pas les lignes selon
  la magnitude du pivot. Les systèmes mal conditionnés ou sensibles au pivotement appartiennent à la voie d'exécution (`solve(A,b)` ci-dessus) ou à une bibliothèque numérique.
- **Matrices de motif uniquement via la voie fichier** : la voie du constructeur pour les matrices de motif a besoin d'un nom de fichier et d'un générateur, donc les macros en ligne les rejettent
  avec une indication ; il n'y a pas de développement automatique de matrices de motif arbitraires (niveau recherche — tout ce qui est hors des formes intégrées est rejeté explicitement
  avec la liste supportée).
- **La validation numérique WGSL est mesurée, mais sur un périphérique logiciel** : `just gpu-verify` compile le noyau émis en SPIR-V, l'exécute via Vulkan et compare les résultats avec la voie f64 du CPU (quatre formules, seize points, pire erreur relative 3.2e-7 sur Mesa lavapipe ; échanger un seul opérateur fait échouer la voie). C'est un vrai chemin de pilote exécutant du vrai SPIR-V, donc il attrape les erreurs d'indexation, de constantes et de précédence — mais il ne mesure **pas** l'arrondi f32 spécifique au vendeur, l'ordonnancement des workgroups ni les bugs de pilote, et aucune exécution sur GPU matériel n'a été faite.
- **La vérification Lean n'a pas réussi** : aucune chaîne d'outils n'est installée sur cet hôte, donc seuls les tests à processus factices ont tourné. Il n'y a aucune revendication que Lean accepte les
  obligations de preuve générées.
- **La publication est une décision séparée et irréversible** : les crates de bibliothèque sont publiables et le dry-run est propre, mais rien n'a été téléversé ; le premier téléversement est le point de non-retour pour un numéro de version. Voir État.
- **Périmètre `no_std`** : `himari`, `himari-latex` et `himari-matlab` sont `no_std + alloc` ; les crates loader/buildhelper/macros/check ne le sont pas.
  Le code généré conserve sa propre voie libm, et les formules complexes ne font pas partie de la voie `no_std`.
- **Pas de dérivées analytiques complexes, pas de variante f32 pour les formules complexes ou à valeur matricielle, pas de dérivée analytique pour les formules à valeur matricielle.**
- **`roots` ne couvre que les degrés 1–2** dans le langage ; les degrés supérieurs seulement via `himari-eval --roots` numériquement.
- LaTeX général (macros/catcodes/composition) — sémantique indécidable ; la composition appartient à la couche de rendu.
- Aucune inférence de sémantique à partir des glyphes, et pas de multiplication `xy` sur la surface MATLAB. Pas de `\sin^2 x`, pas de `.*`, pas de `$…$` (notation
  ambiguë ou purement typographique : l'analyse et le rendu sont deux métiers différents et un seul analyseur ne doit pas faire les deux).
- Pas de JIT à l'exécution sur les chemins temps réel/critiques pour la sécurité ; pas de dépendances AGPL, jamais (evalexpr ≥12 et compagnie sont écartés).
- Les modules **`himari-standards`** sont des calculateurs au niveau de l'article, **pas des revendications de conformité** : ils ne modélisent pas la géométrie des éprouvettes, la sélection du niveau de force,
  le temps de maintien, les budgets d'incertitude, ou les propres règles de rapport des normes, et ils ne sont pas une bibliothèque d'unités générale.
- La voie **`himari-lean`** prouve une petite obligation entière close (un exemple de déterminant 3×3) ; ce n'est pas un démonstrateur de théorèmes général, et sa garde
  d'axiomes est un analyseur de la sortie de Lean elle-même, pas un argument de solidité.
