# Nyckelverkstad

Ett lokalt Windows-program i Rust för lösenord, sifferkoder och API-nycklar.

## Funktioner

- Längd 4–256 tecken.
- Valfria versaler, gemener, siffror och specialtecken. Varje vald grupp finns med minst en gång.
- Enbart versaler, enbart gemener, enbart siffror eller valfri blandning.
- Sifferkoder med snabbval för 4, 6, 8 och 12 siffror.
- API-nycklar i URL-säkert format eller hex med snabbval för 32, 48, 64 och 128 tecken.
- Dölj eller visa resultatet, kopiera till urklipp och rensa det från appen.
- Operativsystemets kryptografiska slumpkälla och urval utan modulo-skevhet.

## Bygg och kör

Installera Rust via rustup och kör sedan i den här mappen:

```powershell
cargo run --release
```

Den fristående programfilen finns därefter i `target\release\nyckelverkstad.exe`.

## Säkerhet

Programmet arbetar lokalt och sparar inte genererade värden på disk. Om du kopierar ett värde ligger det kvar i Windows urklipp tills det ersätts eller rensas. En kort PIN är ofta inte tillräcklig som enda skydd för ett konto.
