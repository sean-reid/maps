# maps

A world map at [maps.dwainosaur.com](https://maps.dwainosaur.com) where each country's area is the quantity being shown: people, tonnes of wheat, tonnes of CO2. Pick a series and the borders stretch and squash to fit it while every country keeps its neighbours. Play the years and the areas grow and shrink.

## Development

```sh
npm install
npm run build      # build the solver with cargo (needs the wasm32 target), then bundle into dist/
npm run dev        # wrangler dev on http://localhost:8787
npm test           # unit tests
npm run test:e2e   # Playwright against wrangler dev
```

## How it runs

One Cloudflare Worker serves the static page and a small `/api`. GitHub Actions deploys it when main is green.
