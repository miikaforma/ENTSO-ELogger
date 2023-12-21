CREATE TABLE "day_ahead_prices"(
   "time" TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
	"currency" TEXT NOT NULL,
	"in_domain" TEXT NOT NULL,
	"out_domain" TEXT NOT NULL,
	"price" REAL NOT NULL,
	"measure_unit" VARCHAR(3) NOT NULL,
	"source" TEXT NULL DEFAULT NULL,
	"tax_percentage" REAL NOT NULL DEFAULT '24',
	UNIQUE (TIME, in_domain, out_domain)
);

SELECT CREATE_HYPERTABLE('day_ahead_prices', BY_RANGE('time'));
