# contributing to zutility

## how to contribute

1. fork the repo
2. create a branch (`git checkout -b feat(scope)/short-description`)
3. commit your changes (`git commit -m 'feat(fe): add thing'`)
4. push (`git push origin feat(scope)/short-description`)
5. open a pull request

## commit convention

use conventional commits with scope prefix:

```
feat(fe): add new utility card
fix(be): handle indexer timeout gracefully
feat(provider): add inlomax airtime support
fix(auth): prevent unauthenticated flash
```

## code style

- no comments in code
- match existing patterns and conventions
- run typecheck and lint before pushing
- test your changes

## reporting issues

use github issues. include steps to reproduce, expected behavior, and environment details.
