db:
	docker run --rm -p3306:3306 --name some-mysql -e MYSQL_ROOT_PASSWORD=test -d mysql:8.0.26 --character-set-server=utf8mb4 --collation-server=utf8mb4_unicode_ci
