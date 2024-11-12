# calp
日本の祝日に対応した簡易calコマンドです。  
**MacOS** および **Linux** 向けのコマンドラインツールです。  

## インストール
あらかじめRustをインストールしておいてください。Rustのインストールは[公式サイト](https://www.rust-lang.org/tools/install)を参考にしてください。
```
cargo build --release
```

作成された`target/release/calp`をパスの通ったディレクトリにコピーします。(e.g. `/usr/local/bin`)  
```
sudo cp target/release/calp /usr/local/bin/
```

## 日本の祝日情報のダウンロード
日本の祝日情報を[内閣府のホームページ](https://www8.cao.go.jp/chosei/shukujitsu/gaiyou.html)からダウンロードします。  
毎年更新されるので、最新のものをダウンロードしてください。  
```
curl -o $HOME/.calp_shuku https://www8.cao.go.jp/chosei/shukujitsu/syukujitsu.csv
```

## ライセンス
このプロジェクトは MIT ライセンスのもとで公開されています。
