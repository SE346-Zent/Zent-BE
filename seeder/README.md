## HOW TO SEED
- Run this command to create the schema
```
cd Zent-BE/
sea-orm-cli migrate up sqlite://data.db?mode=rwc
```
- Run this command to seed data
```
cd seeder/
cargo run -- --db-url "sqlite:/ D:\Ryan\App_project\Zent\Zent-BE\data.db?mode=rwc" --num-users 10
```

Note: The `D:\Ryan\App_project\Zent\Zent-BE\data.db` should be changed to be suitable with your device.
