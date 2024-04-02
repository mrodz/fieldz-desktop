sea-orm-cli.exe migrate refresh
sea-orm-cli.exe generate entity -o entity/src/entities --with-serde both --model-extra-derives PartialOrd,Ord