#!/bin/bash
echo "=== Freezer Monitor Update ==="

echo "1/5 Code aktualisieren..."
git pull || exit 1

echo "2/5 Frontend Pakete installieren..."
cd frontend
npm install || exit 1

echo "3/5 Frontend bauen..."
npm run build || exit 1
cd ..

echo "4/5 Backend kompilieren (dauert ~20 Min)..."
cargo build --release || exit 1

echo "5/5 Server neu starten..."
sudo systemctl restart freezer-monitor

echo ""
echo "=== Update abgeschlossen! ==="
sudo systemctl status freezer-monitor --no-pager