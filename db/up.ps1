sea-orm-cli.exe migrate refresh

if (!$?) {
	exit 1
}

sea-orm-cli.exe generate entity -o entity/src/entities --with-serde both --model-extra-derives PartialOrd,Ord